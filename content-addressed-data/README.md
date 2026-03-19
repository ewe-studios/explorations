# Content-Addressed Data Systems

> A comprehensive exploration of content-addressed storage, routing, and distribution systems

This directory contains deep-dive documentation for content-addressed data systems, with a focus on the `cid-router` project and related technologies.

---

## 📚 Documentation Structure

```
content-addressed-data/
├── exploration.md                          # Comprehensive guide (basic → advanced)
├── README.md                               # This file - index and navigation
├── cid-router-architecture-deep-dive.md    # System architecture overview
├── cid-router-core-deep-dive.md            # Core library deep dive
├── cid-router-iroh-crp-deep-dive.md        # Iroh P2P provider
├── cid-router-azure-crp-deep-dive.md       # Azure Blob Storage provider
├── cid-router-server-deep-dive.md          # HTTP API server
└── cid-router-api-utils-deep-dive.md       # Error handling utilities
```

---

## 🚀 Quick Start

### For Beginners

Start with the **[exploration.md](exploration.md)** guide:
- Part 1: Foundations (hash functions, basic content addressing)
- Part 2: Middle Level (Merkle trees, multihash)
- Part 3: Intermediate (CIDs, IPLD, routing)
- Part 4: Advanced (distributed systems, libp2p)
- Part 5: Building in Rust (implementation guide)

### For Developers

Start with the deep-dive documentation:
1. [Architecture Overview](cid-router-architecture-deep-dive.md) - System overview
2. [Core Library](cid-router-core-deep-dive.md) - Types, traits, and data structures
3. [Providers](#deep-dive-documentation):
   - [Iroh CRP](cid-router-iroh-crp-deep-dive.md) - P2P blob storage
   - [Azure CRP](cid-router-azure-crp-deep-dive.md) - Cloud storage with two-phase indexing
4. [Server API](cid-router-server-deep-dive.md) - HTTP endpoints and authentication
5. [API Utils](cid-router-api-utils-deep-dive.md) - Error handling utilities

---

## 📖 Key Concepts

### Content Addressing

| Concept | Description |
|---------|-------------|
| **Hash Function** | Maps data → fixed-size digest (e.g., SHA-256, BLAKE3) |
| **Multihash** | Self-describing hash (includes algorithm identifier) |
| **CID** | Content Identifier - versioned, codec-aware address |
| **Merkle Tree** | Tree of hashes where root commits to all data |
| **Merkle DAG** | DAG of content-addressed nodes |

### Routing

| Component | Description |
|-----------|-------------|
| **CRP** | CID Route Provider - abstracts storage backends |
| **Route** | Maps CID → retrieval URL |
| **Indexer** | Background process that discovers content |
| **Filter** | Declares which CIDs a provider can handle |

### Storage Backends

| Backend | Type | Hash | Write | P2P |
|---------|------|------|-------|-----|
| **Iroh** | Local + P2P | BLAKE3 | ✅ | ✅ |
| **Azure** | Cloud | Any | ❌ | ❌ |
| **Local** | Filesystem | Any | ✅ | ❌ |

---

## 🏗️ Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                     CID Router System                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────────┐                                                │
│  │   Client    │                                                │
│  └──────┬──────┘                                                │
│         │ HTTP                                                   │
│         ▼                                                        │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │              HTTP API Server                              │   │
│  │  - /v1/routes  (list, query)                             │   │
│  │  - /v1/data    (upload, download)                        │   │
│  │  - /v1/status  (health)                                  │   │
│  └────────────────────────┬────────────────────────────────┘   │
│                           │                                     │
│                           ▼                                     │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │              Application Context                          │   │
│  │  - Core database (SQLite)                                │   │
│  │  - Provider registry                                     │   │
│  │  - Background indexer                                    │   │
│  └────────────────────────┬────────────────────────────────┘   │
│                           │                                     │
│         ┌─────────────────┼─────────────────┐                  │
│         │                 │                 │                   │
│         ▼                 ▼                 ▼                   │
│  ┌─────────────┐  ┌─────────────┐   ┌─────────────┐           │
│  │   Iroh CRP  │  │  Azure CRP  │   │  Local CRP  │           │
│  │  (P2P+Blobs)│  │ (Cloud API) │   │ (Filesystem)│           │
│  └─────────────┘  └─────────────┘   └─────────────┘           │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 🔧 Getting Started with cid-router

### Installation

```bash
# Clone the repository
cd /home/darkvoid/Boxxed/@formulas/src.rust/src.ContentAddressing/cid-router

# Build
cargo build --release

# Or use nix (recommended)
nix develop
make bin.cid-router
```

### Configuration

Create `~/.local/share/cid-router/server.toml`:

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

### Running

```bash
# Start the server
cid-router start

# With custom repo path
cid-router start --repo-path /custom/path

# Generate OpenAPI spec
cid-router openapi ./docs
```

### API Usage

```bash
# Upload content
curl -X POST http://localhost:8080/v1/data \
  -H "Content-Type: application/octet-stream" \
  --data-binary @file.bin

# Download content
curl http://localhost:8080/v1/data/{cid}

# List routes
curl http://localhost:8080/v1/routes

# Get routes for specific CID
curl http://localhost:8080/v1/routes/{cid}

# Check status
curl http://localhost:8080/v1/status
```

---

## 📚 Deep Dive Documentation

### Architecture & Overview

| Document | Description |
|----------|-------------|
| [Architecture](cid-router-architecture-deep-dive.md) | System overview, data flow, configuration |
| [Core Library](cid-router-core-deep-dive.md) | CID types, CRP trait, routes, database layer |

### Providers (CRPs)

| Document | Description |
|----------|-------------|
| [Iroh CRP](cid-router-iroh-crp-deep-dive.md) | P2P blob sync with BLAKE3-only content addressing |
| [Azure CRP](cid-router-azure-crp-deep-dive.md) | Cloud storage with two-phase indexing |

### Server & Utilities

| Document | Description |
|----------|-------------|
| [Server API](cid-router-server-deep-dive.md) | HTTP endpoints, JWT auth, OpenAPI docs |
| [API Utils](cid-router-api-utils-deep-dive.md) | Error handling utilities and types |

---

## 🔍 Code Organization

### Workspace Members

```toml
[workspace]
members = [
    "core",              # Core library
    "crates/api-utils",  # Error handling
    "crps/iroh",         # Iroh provider
    "crps/azure",        # Azure provider
    "server"             # HTTP API
]
```

### Dependency Graph

```
api-utils (no deps)
    │
    ▼
cid-router-core
    │
    ├──────────┬──────────┐
    ▼          ▼          ▼
  iroh       azure      server
```

---

## 🧪 Testing

```bash
# Run all tests
cargo test --all

# Run specific crate tests
cargo test -p cid-router-core
cargo test -p cid-router-server

# Run with output
cargo test -- --nocapture
```

---

## 📖 Related Projects

| Project | Description |
|---------|-------------|
| [Iroh](https://iroh.computer/) | P2P networking and blob sync |
| [IPFS](https://ipfs.tech/) | InterPlanetary File System |
| [IPLD](https://ipld.io/) | InterPlanetary Linked Data |
| [Libp2p](https://libp2p.io/) | P2P networking stack |
| [Multiformats](https://multiformats.io/) | Self-describing formats |

---

## 🎯 Learning Path

### Beginner Path
1. [exploration.md Part 1](exploration.md#part-1-foundations-very-basic)
2. [exploration.md Part 2](exploration.md#part-2-middle-level---hash-functions-and-merkle-trees)
3. [exploration.md Part 3](exploration.md#part-3-intermediate---ipld-cids-and-content-routing)

### Intermediate Path
1. [exploration.md Part 4](exploration.md#part-4-advanced---distributed-content-addressing)
2. [Architecture Deep Dive](cid-router-architecture-deep-dive.md)
3. [Core Library Deep Dive](cid-router-core-deep-dive.md)

### Advanced Path
1. [Iroh CRP Deep Dive](cid-router-iroh-crp-deep-dive.md)
2. [Azure CRP Deep Dive](cid-router-azure-crp-deep-dive.md)
3. [Server API Deep Dive](cid-router-server-deep-dive.md)
4. Implement your own CRP

---

## 📝 Glossary

| Term | Definition |
|------|------------|
| **CID** | Content Identifier - a self-describing content address |
| **CRP** | CID Route Provider - abstracts storage backends |
| **DAG** | Directed Acyclic Graph |
| **DHT** | Distributed Hash Table |
| **IPLD** | InterPlanetary Linked Data |
| **Multihash** | Self-describing hash format |
| **Route** | A mapping from CID to retrieval location |
| **Stub** | Incomplete route (CID not yet computed) |

---

## 🤝 Contributing

When adding new features or CRPs:

1. Follow the existing CRP pattern
2. Add comprehensive tests
3. Update documentation
4. Consider backward compatibility

---

## 📄 License

This documentation follows the license of the cid-router project.

---

## 🔗 Resources

### Specifications
- [CID Specification](https://github.com/multiformats/cid)
- [Multihash Specification](https://github.com/multiformats/multihash)
- [IPLD Data Model](https://ipld.io/docs/data-model/)

### Documentation
- [Iroh Docs](https://iroh.computer/docs)
- [IPFS Docs](https://docs.ipfs.tech/)
- [Libp2p Docs](https://docs.libp2p.io/)

### Communities
- [IPFS Discord](https://discord.gg/ipfs)
- [Iroh Discord](https://discord.gg/iroh)
