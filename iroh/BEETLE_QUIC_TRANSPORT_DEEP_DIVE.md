# Beetle: QUIC Transport Implementation for Iroh

## Overview

Beetle is a comprehensive QUIC-based transport implementation that forms the networking foundation for iroh (Interplanetary File System for Cloud & Mobile). It represents an evolution of the original iroh codebase with a focus on modularity and performance.

**Repository:** https://github.com/n0-computer/beetle
**License:** MIT OR Apache-2.0
**Organization:** number 0 (n0.computer)

## Architecture

### Workspace Structure

Beetle is organized as a Rust workspace with multiple crates:

```
beetle/
├── iroh/              - Core iroh implementation
├── iroh-api/          - Public API definitions
├── iroh-bitswap/      - IPFS bitswap protocol implementation
├── iroh-car/          - Content-addressed repository handling
├── iroh-embed/        - Embedded data handling
├── iroh-gateway/      - HTTP gateway for IPFS content
├── iroh-localops/     - Local operations
├── iroh-metrics/      - Metrics and observability
├── iroh-one/          - Single-binary distribution
├── iroh-p2p/          - P2P networking layer
├── iroh-resolver/     - Content resolution
├── iroh-rpc-client/   - RPC client implementations
├── iroh-rpc-types/    - RPC type definitions
├── iroh-share/        - Content sharing protocols
├── iroh-store/        - Storage layer
├── iroh-unixfs/       - Unix filesystem abstractions
├── iroh-util/         - Utility functions
└── xtask/             - Build automation
```

## Core Components

### Iroh Core

The main `iroh` crate provides:
- QUIC-based peer-to-peer networking
- Content-addressed storage
- Relay-assisted NAT traversal
- Provider/gateway services

### Iroh-Bitswap

Implementation of the IPFS Bitswap protocol for block exchange:

```toml
[package]
name = "iroh-bitswap"
version = "0.2.0"
```

Key features:
- Session-based block requesting
- Peer selection and scoring
- Want-list management
- Message deduplication

### Iroh-Store

Storage abstraction layer providing:
- Block storage with content addressing
- Garbage collection
- Pinning/unpinning semantics
- Batch operations

### Iroh-Gateway

HTTP gateway for serving IPFS content:
- HTTP/HTTPS support
- Range requests
- Content-type detection
- CORS handling
- IPFS path resolution (`/ipfs/`, `/ipns/`)

### Iroh-Metrics

Observability infrastructure:
- Prometheus metrics export
- OpenTelemetry integration
- Tracing support
- Performance monitoring

## QUIC Transport

### Quinn Integration

Beetle uses [quinn](https://github.com/quinn-rs/quinn) as its QUIC implementation:

- **Connection pooling**: Efficient reuse of connections
- **Stream multiplexing**: Multiple logical streams per connection
- **0-RTT handshakes**: Fast reconnection
- **Connection migration**: Maintains connections across IP changes

### ALPN Protocol Negotiation

Applications use ALPN (Application-Layer Protocol Negotiation) to identify protocols:

```rust
const ALPN_IROH_BYTES: &[u8] = b"/iroh-bytes/2";
const ALPN_IROH_DOCS: &[u8] = b"/iroh-docs/1";
```

### NAT Traversal

Beetle implements multi-modal connectivity:

1. **Direct connection**: When peers are on the same network
2. **Hole punching**: Using STUN-like techniques through NATs
3. **Relay fallback**: When direct connection fails

## Content Addressing

### CID (Content Identifier) Support

Implementation of IPFS Content Identifiers:

```rust
use cid::Cid;
use multihash::Multihash;
```

Supported hash functions:
- BLAKE3 (primary)
- SHA2-256
- SHA3-256

### DAG-CBOR

Content serialization using DAG-CBOR for structured data:
- Merkle DAG compatibility
- Deterministic encoding
- IPFS interoperability

## Storage Architecture

### Block Storage

Content is stored as immutable blocks identified by their hash:

```rust
pub struct Block {
    pub cid: Cid,
    pub data: Bytes,
}
```

### UnixFS Abstraction

Filesystem-like abstraction over blocks:
- Files and directories
- Symlinks
- Metadata preservation
- Chunked large files

### CAR (Content Addressed aRchive)

Support for CAR files for:
- Import/export of content
- Backup and restore
- Offline data transfer

## RPC System

### quic-rpc Integration

Beetle uses the `quic-rpc` crate for RPC:

```rust
use quic_rpc::{RpcClient, RpcServer, Service};
```

Features:
- Streaming request/response
- Multiple interaction patterns
- Memory transport for local calls
- QUIC transport for remote calls

### Service Definition

Services define request/response types:

```rust
#[derive(Debug, Clone)]
pub struct IrohService;

impl Service for IrohService {
    type Req = IrohRequest;
    type Res = IrohResponse;
}
```

## Metrics and Observability

### Prometheus Integration

```rust
use iroh_metrics::{
    Counter, Histogram, Registry,
    core::Metric,
};
```

Key metrics:
- Connection counts
- Bytes transferred
- Request latencies
- Cache hit rates

### Tracing

Structured logging with tracing:

```rust
use tracing::{info, debug, error, instrument};

#[instrument(skip(self), level = "debug")]
async fn get_block(&self, cid: &Cid) -> Result<Block> {
    // ...
}
```

## Distributions

### Iroh Cloud

Microservices architecture for datacenter deployment:
- Separated components
- Horizontal scalability
- Kubernetes-friendly

### Iroh One

Single-binary distribution:
- Simplified deployment
- All features in one package
- Easier local development

### Iroh Mobile

iOS and Android libraries:
- Efficient bandwidth usage
- Background sync support
- Battery-conscious operation

## Security Considerations

### TLS Encryption

All QUIC connections use TLS 1.3:
- Forward secrecy
- Certificate validation
- Key rotation

### Capability-Based Access

Content access controlled by capabilities:
- Read capabilities
- Write capabilities
- Delegated access

## Performance Optimizations

### Batch Operations

Bulk operations for efficiency:
- Batch get/put
- Transaction support
- Write coalescing

### Caching

Multi-level caching:
- In-memory cache for hot data
- Disk cache for recent data
- Distributed cache across peers

### Zero-Copy

Minimizing data copies:
- Bytes integration
- Direct buffer writes
- mmap for large files

## Integration with Other n0-computer Projects

### bao-tree

Uses bao-tree for verified streaming:
```rust
use bao_tree::{io::sync::encode_ranges_validated, BlockSize};
```

### quic-rpc

RPC framework for service communication

### iroh-n0des

Node management and discovery

## Development Status

### Current Capabilities

- Content-addressed storage and retrieval
- P2P networking with NAT traversal
- HTTP gateway serving
- Bitswap protocol
- UnixFS support
- CAR import/export

### Future Work

- Improved hole punching success rates
- Enhanced relay infrastructure
- Mobile platform optimizations
- Additional storage backends
- Enhanced encryption options

## Building and Running

### Requirements

- Rust 1.65+
- protoc (for protobuf compilation)
- System headers for rocksdb

### Build Commands

```bash
# Build all crates
cargo build --workspace

# Build optimized release
cargo build --release --profile optimized-release

# Run tests
cargo test --workspace
```

## Testing Strategy

### Unit Tests

Individual component testing with mock data

### Integration Tests

End-to-end protocol testing between nodes

### Property-Based Testing

Using proptest for invariant checking

### Benchmarking

Criterion-based benchmarks for performance tracking

## Conclusion

Beetle represents a comprehensive reimplementation of IPFS concepts with modern Rust practices, focusing on:
- Performance through QUIC
- Modularity through workspace organization
- Observability through metrics and tracing
- Flexibility through multiple distribution formats

It serves as the foundation for the broader n0-computer ecosystem of decentralized applications.

## Related Resources

- [Iroh Documentation](https://iroh.computer/docs)
- [QUIC RFC 9000](https://www.rfc-editor.org/rfc/rfc9000.html)
- [IPFS Specification](https://github.com/ipfs/specs)
- [Beetle Source Repository](https://github.com/n0-computer/beetle)
