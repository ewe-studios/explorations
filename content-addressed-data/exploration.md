---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.ContentAddressing/
repository: N/A - exploration based on cid-router project
explored_at: 2026-03-20
language: Rust
---

# Content-Addressed Data Systems: A Comprehensive Guide

> From first principles to advanced implementation in Rust

## Table of Contents

1. [Introduction](#introduction)
2. [Part 1: Foundations (Very Basic)](#part-1-foundations)
3. [Part 2: Middle Level - Hash Functions and Merkle Trees](#part-2-middle-level)
4. [Part 3: Intermediate - IPLD, CIDs, and Content Routing](#part-3-intermediate)
5. [Part 4: Advanced - Distributed Content Addressing](#part-4-advanced)
6. [Part 5: Building from Scratch in Rust](#part-5-rust-implementation)

---

## Introduction

Content-addressed data is a paradigm where data is referenced by its content rather than its location. This simple shift has profound implications for:

- **Deduplication**: Identical content always produces the same address
- **Integrity verification**: The address proves the content hasn't been tampered with
- **Distribution**: Content can come from anywhere, not just a canonical source
- **Permanence**: Content outlives any specific storage location

This guide walks through everything from the basic concepts to building a complete content-addressed storage system in Rust, based on learnings from the `cid-router` project and the broader IPFS/libp2p ecosystem.

---

## Part 1: Foundations (Very Basic)

### 1.1 What is Content Addressing?

#### Location-Based Addressing (Traditional)

In traditional systems, we reference data by **where** it lives:

```
https://example.com/files/document.pdf
/usr/local/bin/program
C:\Users\Alice\Photos\vacation.jpg
```

**Problems:**
- If the file moves, the reference breaks (link rot)
- Same file in two places has two different addresses
- No way to verify the content is correct
- Trust depends entirely on the source

#### Content-Based Addressing

In content-addressed systems, we reference data by **what** it is:

```
bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi
QmXoypizjW3WknFiJnKLwHCnL72vedxjQkDDP1mXWo6uco
baaiaqcazqapazqzaqzaqzaqzaqzaqzaqzaqzaqzaqzaqzaqz
```

**Benefits:**
- Same content always has the same address (deduplication)
- Address proves content integrity (security)
- Content can come from anywhere (distribution)
- Content is permanent and location-independent

### 1.2 The Core Mechanism: Cryptographic Hash Functions

A **hash function** takes any input and produces a fixed-size output called a **digest** or **hash**.

```
SHA256("Hello") = 185f8db32271fe25f561a6fc938b2e264306ec304eda518007d1764826381969
SHA256("Hello") = 185f8db32271fe25f561a6fc938b2e264306ec304eda518007d1764826381969 (always!)
SHA256("hello") = 2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824 (different!)
```

**Key Properties:**

1. **Deterministic**: Same input always produces the same hash
2. **Fixed size**: Any input produces the same length output (e.g., 256 bits for SHA-256)
3. **Fast to compute**: Hashing is computationally efficient
4. **Pre-image resistant**: Given a hash, you can't recover the original input
5. **Collision resistant**: It's infeasible to find two inputs with the same hash
6. **Avalanche effect**: Changing one bit in input changes ~50% of output bits

### 1.3 Common Hash Functions

| Algorithm | Output Size | Speed | Security | Use Cases |
|-----------|-------------|-------|----------|-----------|
| SHA-1 | 160 bits | Fast | Broken | Git (legacy) |
| SHA-256 | 256 bits | Medium | Secure | Bitcoin, IPFS |
| BLAKE3 | 256 bits (variable) | Very Fast | Secure | Modern systems |
| MurmurHash3 | 32-128 bits | Very Fast | Not cryptographic | Hash tables |

### 1.4 Simple Example: Hash-Based File Storage

Let's create the simplest content-addressed storage:

```
Storage Layout:
  ./storage/
    18/5f/8d/  (first bytes of hash as directory)
      185f8db32271fe25f561a6fc938b2e264306ec304eda518007d1764826381969 (full hash as filename)
```

**Storing data:**
1. Compute hash of data
2. Use hash as the filename
3. Store the data

**Retrieving data:**
1. Know the hash you want
2. Look up the file by hash
3. Verify the content matches the hash

---

## Part 2: Middle Level - Hash Functions and Merkle Trees

### 2.1 Deep Dive: How Hash Functions Work

#### SHA-256 Internals

SHA-256 processes data in 512-bit blocks using the Merkle-Damgård construction:

```
Input: "Hello World" (arbitrary length)

Step 1: Padding
  "Hello World" + [1] + [0000...] + [length as 64-bit integer]
  Result: Exactly 512 bits (or multiple)

Step 2: Initialize hash values (8 words from prime fractional parts)
  h0 = 0x6a09e667, h1 = 0xbb67ae85, ..., h7 = 0x5be0cd19

Step 3: Process each 512-bit block through 64 rounds
  For each round:
  - Mix the block with constants using bitwise operations
  - Update working variables (a, b, c, d, e, f, g, h)
  - Add results to hash values

Step 4: Concatenate h0h1h2h3h4h5h6h7
  Output: 256-bit (32 byte) hash
```

#### BLAKE3: Modern and Fast

BLAKE3 improves on SHA-256 with:

- **Parallelism**: Uses a tree structure for parallel hashing
- **Variable output**: Can produce any length output
- **SIMD optimizations**: Uses CPU vector instructions
- **Simpler design**: Fewer rounds, faster execution

```
BLAKE3 Tree Structure (for large files):

File: [Chunk 0][Chunk 1][Chunk 2][Chunk 3]
              │          │          │          │
              ▼          ▼          ▼          ▼
           H(0)       H(1)       H(2)       H(3)
              │          │          │          │
              └──────────┴──────────┴──────────┘
                         │
                         ▼
                      H(root)
                         │
                         ▼
                    Final Output
```

### 2.2 Multihash: Self-Describing Hashes

Different systems use different hash functions. **Multihash** encodes the hash function identifier along with the hash:

```
Multihash Format:
  [code varint][size varint][digest bytes]

Example (SHA-256 hash of "hello"):
  12 20 2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824
  │  │  └─────────────────────────────────────────────────────────────┘
  │  │  └─────────────────────────────────────────────────────────────┘
  │  │  └─────────────────────────────────────────────────────────────┘
  │  └─ Digest length (32 bytes = 64 hex chars)
  └─ Hash code (0x12 = SHA-256)

Common Codes:
  0x11 = SHA-1
  0x12 = SHA-256
  0x1e = BLAKE3
  0xb240 = SHA3-256
```

**Why Multihash?**
- Future-proof: new hash functions can be added
- Interoperability: systems can understand each other's hashes
- Verification: recipients know which algorithm to use

### 2.3 Merkle Trees: Hashing Data Structures

A **Merkle Tree** is a tree where every node is labeled with the hash of its children.

```
Simple Merkle Tree:

           Root = H(H(A) || H(B))
                 /                  \
         H(A) = H(data A)    H(B) = H(data B)
              /      \              /      \
            A1       A2           B1       B2
```

**Properties:**
- **Tamper-evident**: Changing any leaf changes the root
- **Efficient proofs**: Prove a leaf is in the tree with log(n) hashes
- **Parallel verification**: Each subtree can be verified independently

#### Merkle Tree Use Cases

1. **Blockchain**: Bitcoin uses Merkle trees to commit to transactions
2. **Filesystems**: IPFS, Git, and ZFS use Merkle DAGs
3. **Databases**: Certificate Transparency, Amazon QLDB

### 2.4 Merkle DAGs (Directed Acyclic Graphs)

IPFS extends Merkle trees to **Merkle DAGs** where nodes can have multiple parents:

```
Merkle DAG Example:

      Root (Directory)
     /    |    \
   File1  File2 File3
   /  \        |
 Chunk Chunk  Chunk
```

**Benefits:**
- Content deduplication across the graph
- Efficient partial verification
- Natural representation of nested data

---

## Part 3: Intermediate - IPLD, CIDs, and Content Routing

### 3.1 CID: Content Identifier

A **CID** (Content Identifier) is a self-describing content address that includes:
- The multihash of the content
- The codec (format) of the content
- The CID version

```
CIDv1 Format (binary):
  [version][codec][multihash]
     │        │         │
     │        │         └─ Multihash (code + size + digest)
     │        └────────── Multicodec (e.g., raw, dag-cbor, dag-json)
     └─────────────────── CID version (0x01)

CIDv1 String (Base32 encoded):
  bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi
  │   └────────────────────────────────────────────────────┘
  │  Multibase prefix (b = base32)
  └─ CIDv1 marker (ba)

CID Components:
┌─────────────┬──────────────┬─────────────────────────────────────┐
│   Version   │    Codec     │            Multihash                │
│   (1 byte)  │  (varint)    │  (code + size + digest bytes)       │
├─────────────┼──────────────┼─────────────────────────────────────┤
│    0x01     │   0x55(raw)  │  0x1e(BLAKE3) + 0x20 + [32 bytes]   │
│    0x01     │   0x71(dag-cbor) │  0x12(SHA256) + 0x20 + [32 bytes] │
└─────────────┴──────────────┴─────────────────────────────────────┘
```

**Common Codecs:**
- `raw` (0x55): Raw binary data
- `dag-cbor` (0x71): CBOR-encoded DAG nodes
- `dag-json` (0x0129): JSON-encoded DAG nodes
- `git-raw` (0x78): Git object format

### 3.2 IPLD: InterPlanetary Linked Data

**IPLD** (InterPlanetary Linked Data) is a data model that unifies content-addressed data structures.

```
IPLD Data Model:

IPLD Value =
  | null
  | bool
  | integer
  | float
  | string
  | bytes
  | list<IPLD Value>
  | map<string, IPLD Value>
  | Link<CID>  ← This is the magic!

Example IPLD Document (in dag-cbor):
{
  "name": "Alice",
  "age": 30,
  "friend": {"/": "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi"}
}
                                         └─ CID link to another IPLD node
```

**IPLD Selectors** allow you to traverse linked data:

```
Selector Path:
  /name          → "Alice"
  /friend/name   → Follows the link, then gets "name"
  /0             → First element of a list
  /*             → All children (recursive)
```

### 3.3 Content Routing: Finding Data by CID

Once you have a CID, how do you actually **get** the content? This is the content routing problem.

```
Content Routing Architecture:

  ┌──────────────┐
  │   Client     │ Has CID: bafy...
  └──────┬───────┘
         │ "Who has bafy...?"
         ▼
  ┌──────────────┐
  │  DHT/Index   │ Maps CIDs to providers
  └──────┬───────┘
         │ "Provider X has it at URL Y"
         ▼
  ┌──────────────┐
  │  Provider X  │ Serves the actual content
  └──────────────┘
```

#### CID Router Pattern (from cid-router project)

The `cid-router` project implements a flexible content routing architecture:

```
┌─────────────────────────────────────────────────────────────┐
│                     CID Router Server                        │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │   Azure     │  │    Iroh     │  │    Iroh     │         │
│  │   CRP       │  │    CRP      │  │    CRP      │         │
│  │  Provider   │  │  Provider   │  │  Provider   │         │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘         │
│         │                │                │                 │
│         └────────────────┴────────────────┘                 │
│                          │                                  │
│                   ┌──────▼──────┐                           │
│                   │  Indexer    │                           │
│                   │  (Periodic  │                           │
│                   │   reindex)  │                           │
│                   └──────┬──────┘                           │
│                          │                                  │
│                   ┌──────▼──────┐                           │
│                   │   SQLite    │                           │
│                   │   Database  │                           │
│                   │  (Routes)   │                           │
│                   └──────┬──────┘                           │
│                          │                                  │
│  ┌───────────────────────▼───────────────────────┐         │
│  │              API Layer                         │         │
│  │  GET /cid/{cid} → Returns routes for CID      │         │
│  │  POST /routes   → Register new route          │         │
│  └───────────────────────────────────────────────┘         │
│                                                             │
└─────────────────────────────────────────────────────────────┘

Request Flow:
1. Client requests CID from API
2. API queries database for routes
3. Database returns provider URLs
4. Client retrieves content from provider
```

### 3.4 CRP: CID Route Provider

A **CRP** (CID Route Provider) is a pluggable backend that can serve content for CIDs.

```rust
// CRP Trait (from cid-router)
#[async_trait]
pub trait Crp: Send + Sync + Debug {
    fn provider_id(&self) -> String;
    fn provider_type(&self) -> ProviderType;
    async fn reindex(&self, cx: &Context) -> Result<()>;

    fn capabilities(&self) -> CrpCapabilities;
    fn cid_filter(&self) -> CidFilter;
}

// A CRP can resolve routes (get content)
#[async_trait]
pub trait RouteResolver {
    async fn get_bytes(
        &self,
        route: &Route,
        auth: Option<Bytes>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<Bytes>> + Send>>>;
}

// A CRP can write blobs
#[async_trait]
pub trait BlobWriter {
    async fn put_blob(
        &self,
        auth: Option<Bytes>,
        cid: &Cid,
        data: &[u8],
    ) -> Result<()>;
}
```

### 3.5 Route Structure

A **Route** connects a CID to a retrieval location:

```rust
pub struct Route {
    pub id: Uuid,
    pub created_at: DateTime,
    pub verified_at: DateTime,
    pub provider_id: String,
    pub provider_type: ProviderType,
    pub url: String,          // Where to fetch the content
    pub cid: Cid,             // The content identifier
    pub size: u64,            // Size in bytes
    pub multicodec: Codec,    // Content format
    pub creator: PublicKey,   // Who created this route
    pub signature: Vec<u8>,   // Signature proving authenticity
}
```

**Route Lifecycle:**

```
1. Create Stub          2. Compute CID        3. Complete Route
   (before content)       (after upload)        (signed & verified)
   ┌─────────────┐       ┌─────────────┐       ┌─────────────┐
   │ url: /blob  │       │ url: /blob  │       │ url: /blob  │
   │ cid: null   │  →    │ cid: CALC!  │  →    │ cid: CALC!  │
   │ size: null  │       │ size: 1024  │       │ size: 1024  │
   │ signature:∅ │       │ signature:∅ │       │ signature:✓ │
   └─────────────┘       └─────────────┘       └─────────────┘
```

### 3.6 CID Filters

**CID Filters** allow CRPs to declare which CIDs they can handle:

```rust
pub enum CidFilter {
    None,                              // Accepts all CIDs
    MultihashCodeFilter(CodeFilter),   // Filter by hash algorithm
    CodecFilter(CodeFilter),           // Filter by codec
    And(Vec<Self>),                    // Combined conditions
    Or(Vec<Self>),
    Not(Box<Self>),
}

// Example: Only accept BLAKE3 hashes
let filter = CidFilter::MultihashCodeFilter(CodeFilter::Eq(0x1e));

// Example: Accept BLAKE3 or SHA-256
let filter = CidFilter::MultihashCodeFilter(
    CodeFilter::Eq(0x1e) | CodeFilter::Eq(0x12)
);
```

---

## Part 4: Advanced - Distributed Content Addressing

### 4.1 Iroh: P2P Content Distribution

**Iroh** is a protocol for building distributed applications with content-addressed data.

```
Iroh Architecture:

┌─────────────────────────────────────────────────────────────┐
│                        Iroh Node                             │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │   SecretKey  │  │  PublicKey   │  │  NodeId      │      │
│  │  (Identity)  │──│  (Derived)   │──│  (Network)   │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │                  Blob Store                           │   │
│  │  ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐                    │   │
│  │  │Blo b│ │Blo b│ │Blo b│ │Blo b│  (Content-addressed)│   │
│  │  │ ABC │ │ DEF │ │ GHI │ │ JKL │                    │   │
│  │  └─────┘ └─────┘ └─────┘ └─────┘                    │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │   QUIC       │  │   TCP        │  │   WebRTC     │      │
│  │  Transport   │  │  Transport   │  │  Transport   │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

**Iroh Blob Store:**
- Content-addressed storage using BLAKE3
- Incremental verification during transfer
- Streaming support for large blobs

### 4.2 Libp2p: The P2P Network Stack

**Libp2p** provides the networking layer for content-addressed systems:

```
Libp2p Stack:

┌─────────────────────────────────────────┐
│          Application Layer              │
│         (IPFS, Iroh, etc.)              │
├─────────────────────────────────────────┤
│              Peer Discovery             │
│    (mDNS, Kademlia DHT, DNS-SD)         │
├─────────────────────────────────────────┤
│               Routing                   │
│         (Kademlia DHT)                  │
├─────────────────────────────────────────┤
│              Transport                  │
│    (TCP, QUIC, WebSocket, WebRTC)       │
├─────────────────────────────────────────┤
│          Secure Channel                 │
│         (TLS, Noise, PLAIN)             │
├─────────────────────────────────────────┤
│        Peer Identity                    │
│      (Ed25519, Secp256k1)               │
└─────────────────────────────────────────┘
```

### 4.3 Kademlia DHT: Distributed Hash Table

**Kademlia** is the DHT algorithm used by IPFS, Ethereum, and BitTorrent.

```
Kademlia XOR Distance:

distance(A, B) = A XOR B

Example (4-bit IDs):
  Node A: 1010
  Node B: 1100
  Distance: 0110 (= 6)

Properties:
- Symmetric: distance(A,B) = distance(B,A)
- Triangle inequality holds
- Creates a metric space for routing

K-Buckets (Routing Table):

Node ID: 1010

Bucket 0 (distance 1-1):   Nodes starting with 101X
Bucket 1 (distance 2-3):   Nodes starting with 10XX (not 101X)
Bucket 2 (distance 4-7):   Nodes starting with 1XXX (not 10XX)
Bucket 3 (distance 8-15):  Nodes starting with 0XXX

Each bucket stores up to k nodes (typically 20)
```

**Kademlia RPC:**

```
FIND_NODE(id): Return nodes in routing table closest to id

STORE(key, value): Store value under key

FIND_VALUE(key): Return value or closest nodes to key

Message Flow - Finding Content:

Node A wants content with key K

A → B (closest in A's table): "FIND_VALUE K"
B → A: "I don't have it, try C, D, E (closer to K)"
A → C: "FIND_VALUE K"
C → A: "Here's the value!" (or more closer nodes)
```

### 4.4 Bitswap: Content Exchange Protocol

**Bitswap** is the trade protocol used by IPFS:

```
Bitswap State Machine:

┌──────────────┐
│  Want-List   │  CIDs I want
│  [A, B, C]   │
└──────┬───────┘
       │
       ▼
┌──────────────┐     ┌──────────────┐
│    Have?     │────▶│   Send       │
│  (check local)│    │   Blocks     │
└──────────────┘     └──────────────┘
       │
       ▼
┌──────────────┐     ┌──────────────┐
│  Send Want   │────▶│  Receive     │
│   to peers   │     │   Blocks     │
└──────────────┘     └──────────────┘

Bitswap Message:
┌─────────────────────────────────────┐
│         Want-List (up to 1024)      │
│  - Full block wants (I need this)   │
│  - Have wants (Do you have this?)   │
├─────────────────────────────────────┤
│           Blocks to send            │
│  - (CID, data) pairs                │
├─────────────────────────────────────┤
│          Cancel wants               │
│  - CIDs I no longer need            │
└─────────────────────────────────────┘
```

### 4.5 Graphsync: Queryable Content

**Graphsync** extends Bitswap for IPLD graphs:

```
Graphsync Request:

{
  "root": "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
  "selector": {
    ".": {
      ">": {
        "/": "links",
        ".": {
          ">": {
            "/": 0
          }
        }
      }
    }
  }
}

This requests:
1. Start at root CID
2. Follow "links" field
3. Get first element of the list
4. Return all blocks in the path
```

---

## Part 5: Building from Scratch in Rust

### 5.1 Project Structure

Following the `cid-router` pattern:

```
content-addressed-rust/
├── Cargo.toml
├── core/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── cid.rs          # CID types and utilities
│       ├── multihash.rs    # Hash function support
│       ├── codec.rs        # Content codecs
│       ├── route.rs        # Content routing
│       ├── crp.rs          # CRP trait
│       ├── db.rs           # Storage backend
│       └── context.rs      # Application context
├── crps/
│   ├── iroh/              # Iroh CRP implementation
│   └── local/             # Local filesystem CRP
└── server/
    └── src/
        └── main.rs        # HTTP API server
```

### 5.2 Implementing Multihash Support

```rust
// core/src/multihash.rs
use multihash::{Multihash, MultihashDigest};

// Hash algorithm codes
pub const SHA256: u64 = 0x12;
pub const BLAKE3: u64 = 0x1e;

pub fn hash_sha256(data: &[u8]) -> Multihash<64> {
    use sha2::Sha256;
    Sha256::digest(data)
}

pub fn hash_blake3(data: &[u8]) -> Multihash<64> {
    use blake3::Hasher;
    let mut hasher = blake3::Hasher::new();
    hasher.update(data);
    let hash = hasher.finalize();
    Multihash::wrap(BLAKE3, hash.as_bytes()).unwrap()
}

// Verify data matches a hash
pub fn verify(data: &[u8], expected: &Multihash<64>) -> bool {
    let code = expected.code();
    match code {
        SHA256 => hash_sha256(data) == *expected,
        BLAKE3 => hash_blake3(data) == *expected,
        _ => false,
    }
}
```

### 5.3 Implementing CID Types

```rust
// core/src/cid.rs
use cid::Cid;
use serde::{Deserialize, Serialize};

// Codec codes
pub const RAW: u64 = 0x55;
pub const DAG_CBOR: u64 = 0x71;
pub const DAG_JSON: u64 = 0x0129;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Codec {
    Raw,
    DagCbor,
    DagJson,
}

impl Codec {
    pub fn code(&self) -> u64 {
        match self {
            Codec::Raw => RAW,
            Codec::DagCbor => DAG_CBOR,
            Codec::DagJson => DAG_JSON,
        }
    }
}

// Create a CID for raw data with BLAKE3
pub fn create_blake3_cid(data: &[u8]) -> Cid {
    let hash = crate::multihash::hash_blake3(data);
    Cid::new_v1(RAW, hash)
}

// Create a CID for DAG-CBOR data
pub fn create_dag_cbor_cid<T: Serialize>(value: &T) -> Cid {
    let encoded = serde_cbor::to_vec(value).unwrap();
    let hash = crate::multihash::hash_blake3(&encoded);
    Cid::new_v1(DAG_CBOR, hash)
}
```

### 5.4 Building a Content Store

```rust
// core/src/store.rs
use std::path::{Path, PathBuf};
use tokio::fs;
use cid::Cid;

pub struct ContentStore {
    root: PathBuf,
}

impl ContentStore {
    pub async fn new(root: impl AsRef<Path>) -> std::io::Result<Self> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root).await?;
        Ok(Self { root })
    }

    /// Store content and return its CID
    pub async fn put(&self, data: &[u8]) -> std::io::Result<Cid> {
        let cid = crate::cid::create_blake3_cid(data);
        let path = self.cid_path(&cid);

        // Create directory structure from first bytes of hash
        let hash_str = cid.hash().to_string();
        let dir = path.parent().unwrap();
        fs::create_dir_all(dir).await?;

        // Write content
        fs::write(path, data).await?;

        Ok(cid)
    }

    /// Retrieve content by CID
    pub async fn get(&self, cid: &Cid) -> std::io::Result<Vec<u8>> {
        let path = self.cid_path(cid);
        let data = fs::read(path).await?;

        // Verify integrity
        let expected_hash = cid.hash();
        let actual_hash = crate::multihash::hash_blake3(&data);
        if expected_hash != &actual_hash {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Content hash mismatch",
            ));
        }

        Ok(data)
    }

    /// Check if content exists
    pub async fn has(&self, cid: &Cid) -> bool {
        self.cid_path(cid).exists()
    }

    fn cid_path(&self, cid: &Cid) -> PathBuf {
        let hash = cid.hash().to_string();
        // Use first 4 chars as directory for better FS performance
        self.root.join(&hash[0..2]).join(&hash[2..4]).join(&hash)
    }
}
```

### 5.5 Implementing the CRP Trait

```rust
// core/src/crp.rs
use async_trait::async_trait;
use bytes::Bytes;
use cid::Cid;
use std::pin::Pin;
use futures::Stream;
use anyhow::Result;

#[derive(Debug, Clone)]
pub enum ProviderType {
    Local,
    Iroh,
    Azure,
}

#[async_trait]
pub trait Crp: Send + Sync + std::fmt::Debug {
    fn provider_id(&self) -> String;
    fn provider_type(&self) -> ProviderType;
    async fn reindex(&self, cx: &Context) -> Result<()>;
    fn capabilities(&self) -> CrpCapabilities;
    fn cid_filter(&self) -> CidFilter;
}

pub struct CrpCapabilities<'a> {
    pub route_resolver: Option<&'a dyn RouteResolver>,
    pub blob_writer: Option<&'a dyn BlobWriter>,
}

#[async_trait]
pub trait RouteResolver {
    async fn get_bytes(
        &self,
        cid: &Cid,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<Bytes>> + Send>>,
        Box<dyn std::error::Error + Send + Sync>,
    >;
}

#[async_trait]
pub trait BlobWriter {
    async fn put_blob(&self, cid: &Cid, data: &[u8]) -> Result<()>;
}
```

### 5.6 Local Filesystem CRP

```rust
// crps/local/src/lib.rs
use async_trait::async_trait;
use bytes::Bytes;
use cid::Cid;
use cid_router_core::{
    cid_filter::{CidFilter, CodeFilter},
    crp::{BlobWriter, Crp, CrpCapabilities, ProviderType, RouteResolver},
    Context,
};
use std::{
    io,
    path::PathBuf,
    pin::Pin,
};
use futures::Stream;
use tokio::fs;

#[derive(Debug)]
pub struct LocalCrp {
    root: PathBuf,
    writeable: bool,
}

impl LocalCrp {
    pub async fn new(root: PathBuf, writeable: bool) -> io::Result<Self> {
        tokio::fs::create_dir_all(&root).await?;
        Ok(Self { root, writeable })
    }

    fn blob_path(&self, cid: &Cid) -> PathBuf {
        let hash = cid.hash().to_string();
        self.root.join(&hash[0..2]).join(&hash[2..4]).join(&hash)
    }
}

#[async_trait]
impl Crp for LocalCrp {
    fn provider_id(&self) -> String {
        "local".to_string()
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Local
    }

    async fn reindex(&self, _cx: &Context) -> Result<(), anyhow::Error> {
        // Walk directory and update index
        Ok(())
    }

    fn capabilities(&self) -> CrpCapabilities {
        CrpCapabilities {
            route_resolver: Some(self),
            blob_writer: if self.writeable { Some(self) } else { None },
        }
    }

    fn cid_filter(&self) -> CidFilter {
        // Only accept BLAKE3
        CidFilter::MultihashCodeFilter(CodeFilter::Eq(0x1e))
    }
}

#[async_trait]
impl BlobWriter for LocalCrp {
    async fn put_blob(&self, cid: &Cid, data: &[u8]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.writeable {
            return Err("CRP is not writeable".into());
        }

        let path = self.blob_path(cid);
        let parent = path.parent().unwrap();
        fs::create_dir_all(parent).await?;
        fs::write(&path, data).await?;

        Ok(())
    }
}

#[async_trait]
impl RouteResolver for LocalCrp {
    async fn get_bytes(
        &self,
        cid: &Cid,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<Bytes>> + Send>>, Box<dyn std::error::Error + Send + Sync>> {
        let path = self.blob_path(cid);
        let data = fs::read(path).await?;
        let stream = futures::stream::once(async move { Ok(Bytes::from(data)) });
        Ok(Box::pin(stream))
    }
}
```

### 5.7 Database for Routes

```rust
// core/src/db.rs
use rusqlite::{params, Connection, Result};
use cid::Cid;
use uuid::Uuid;
use time::OffsetDateTime;

pub struct Db {
    conn: Connection,
}

impl Db {
    pub fn open(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.create_tables()?;
        Ok(db)
    }

    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.create_tables()?;
        Ok(db)
    }

    fn create_tables(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS routes (
                id TEXT PRIMARY KEY,
                cid BLOB NOT NULL,
                url TEXT NOT NULL,
                provider_id TEXT NOT NULL,
                provider_type TEXT NOT NULL,
                size INTEGER,
                created_at TEXT,
                verified_at TEXT,
                creator BLOB,
                signature BLOB,
                UNIQUE(cid, provider_id, url)
            )",
            [],
        )?;
        Ok(())
    }

    pub fn insert_route(
        &self,
        cid: &Cid,
        url: &str,
        provider_id: &str,
        provider_type: &str,
        size: u64,
    ) -> Result<Uuid> {
        let id = Uuid::new_v4();
        let now = OffsetDateTime::now_utc();

        self.conn.execute(
            "INSERT INTO routes (id, cid, url, provider_id, provider_type, size, created_at, verified_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                id.to_string(),
                cid.to_bytes(),
                url,
                provider_id,
                provider_type,
                size as i64,
                now.to_string(),
                now.to_string(),
            ],
        )?;

        Ok(id)
    }

    pub fn routes_for_cid(&self, cid: &Cid) -> Result<Vec<Route>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, cid, url, provider_id, provider_type, size, created_at, verified_at
             FROM routes WHERE cid = ?1",
        )?;

        let rows = stmt.query_map(params![cid.to_bytes()], |row| {
            Ok(Route {
                id: row.get(0)?,
                cid: row.get(1)?,
                url: row.get(2)?,
                provider_id: row.get(3)?,
                provider_type: row.get(4)?,
                size: row.get(5)?,
                created_at: row.get(6)?,
                verified_at: row.get(7)?,
            })
        })?;

        rows.collect()
    }
}

#[derive(Debug)]
pub struct Route {
    pub id: String,
    pub cid: Vec<u8>,
    pub url: String,
    pub provider_id: String,
    pub provider_type: String,
    pub size: i64,
    pub created_at: String,
    pub verified_at: String,
}
```

### 5.8 HTTP API Server

```rust
// server/src/main.rs
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use cid::Cid;
use std::sync::Arc;

#[derive(Clone)]
struct AppState {
    db: Arc<Db>,
    providers: Arc<Vec<Arc<dyn Crp>>>,
}

#[tokio::main]
async fn main() {
    // Initialize database
    let db = Arc::new(Db::in_memory().unwrap());

    // Initialize providers
    let local_crp = Arc::new(LocalCrp::new("./storage".into(), true).await.unwrap());
    let providers = Arc::new(vec![local_crp as Arc<dyn Crp>]);

    let state = AppState { db, providers };

    // Build router
    let app = Router::new()
        .route("/cid/:cid", get(get_cid))
        .route("/routes", post(create_route))
        .route("/blobs/:cid", get(get_blob))
        .with_state(state);

    // Run server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn get_cid(
    State(state): State<AppState>,
    Path(cid_str): Path<String>,
) -> Result<Json<Vec<RouteResponse>>, StatusCode> {
    let cid = Cid::from_str(&cid_str).map_err(|_| StatusCode::BAD_REQUEST)?;

    let routes = state.db.routes_for_cid(&cid).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let responses: Vec<RouteResponse> = routes
        .into_iter()
        .map(|r| RouteResponse {
            url: r.url,
            provider_id: r.provider_id,
            size: r.size as u64,
        })
        .collect();

    Ok(Json(responses))
}

async fn get_blob(
    State(state): State<AppState>,
    Path(cid_str): Path<String>,
) -> Result<Response<Body>, StatusCode> {
    let cid = Cid::from_str(&cid_str).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Find a provider that can resolve this CID
    for provider in state.providers.iter() {
        if let Some(resolver) = provider.capabilities().route_resolver {
            if provider.cid_filter().is_match(&cid) {
                let stream = resolver.get_bytes(&cid).await
                    .map_err(|_| StatusCode::NOT_FOUND)?;

                return Ok(Response::new(Body::wrap_stream(stream)));
            }
        }
    }

    Err(StatusCode::NOT_FOUND)
}

#[derive(Serialize)]
struct RouteResponse {
    url: String,
    provider_id: String,
    size: u64,
}

#[derive(Deserialize)]
struct CreateRouteRequest {
    cid: String,
    url: String,
    size: u64,
}

async fn create_route(
    State(state): State<AppState>,
    Json(req): Json<CreateRouteRequest>,
) -> Result<StatusCode, StatusCode> {
    let cid = Cid::from_str(&req.cid).map_err(|_| StatusCode::BAD_REQUEST)?;

    state.db.insert_route(
        &cid,
        &req.url,
        "local",
        "Local",
        req.size,
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::CREATED)
}
```

### 5.9 Indexer: Periodic Reindexing

```rust
// core/src/indexer.rs
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use log::{info, warn};

pub struct Indexer {
    _task: tokio::task::JoinHandle<()>,
}

impl Indexer {
    pub fn spawn(
        interval_seconds: u64,
        cx: Context,
        providers: Vec<Arc<dyn Crp>>,
    ) -> Self {
        let task = tokio::spawn(async move {
            info!("Starting indexer for {} providers", providers.len());

            loop {
                for provider in &providers {
                    info!(
                        "Reindexing provider {}:{}...",
                        provider.provider_type(),
                        provider.provider_id()
                    );

                    if let Err(err) = provider.reindex(&cx).await {
                        warn!(
                            "Error reindexing provider {}:{}: {}",
                            provider.provider_type(),
                            provider.provider_id(),
                            err
                        );
                    }
                }

                sleep(Duration::from_secs(interval_seconds)).await;
            }
        });

        Self { _task: task }
    }
}
```

### 5.10 Putting It All Together

```rust
// examples/basic_usage.rs
use content_addressed_rust::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize context with in-memory database
    let cx = Context::mem().await?;

    // Create local CRP
    let local = LocalCrp::new("./storage".into(), true).await?;

    // Store some content
    let data = b"Hello, content-addressed world!";
    let cid = cid::create_blake3_cid(data);

    // Write to local storage
    if let Some(writer) = local.capabilities().blob_writer {
        writer.put_blob(&cid, data).await?;
    }

    // Create a route
    db.insert_route(
        &cid,
        "local://storage",
        &local.provider_id(),
        "local",
        data.len() as u64,
    )?;

    // Retrieve content
    let routes = db.routes_for_cid(&cid)?;
    println!("Found {} routes for CID {}", routes.len(), cid);

    // Resolve through CRP
    if let Some(resolver) = local.capabilities().route_resolver {
        let stream = resolver.get_bytes(&cid).await?;
        let bytes = stream
            .into_future()
            .await
            .0
            .unwrap()
            .unwrap();
        println!("Retrieved: {}", String::from_utf8_lossy(&bytes));
    }

    Ok(())
}
```

---

## Appendix A: Reference Implementation Checklist

Building a complete content-addressed system:

- [ ] **Multihash support**
  - [ ] SHA-256 implementation
  - [ ] BLAKE3 implementation
  - [ ] Multihash encoding/decoding

- [ ] **CID implementation**
  - [ ] CIDv0 and CIDv1 support
  - [ ] Multibase encoding (base32, base58btc)
  - [ ] Codec support (raw, dag-cbor, dag-json)

- [ ] **Storage layer**
  - [ ] Content-addressed file store
  - [ ] Integrity verification on read
  - [ ] Batch operations

- [ ] **Routing layer**
  - [ ] CRP trait definition
  - [ ] Route database schema
  - [ ] Route signing/verification

- [ ] **Network layer**
  - [ ] Libp2p integration (optional)
  - [ ] HTTP API
  - [ ] Peer discovery

- [ ] **IPLD support** (optional)
  - [ ] DAG traversal
  - [ ] Selector implementation
  - [ ] Graphsync protocol

---

## Appendix B: Further Reading

### Specifications
- [CID Specification](https://github.com/multiformats/cid)
- [Multihash Specification](https://github.com/multiformats/multihash)
- [Multicodec Specification](https://github.com/multiformats/multicodec)
- [IPLD Data Model](https://ipld.io/docs/data-model/)
- [Libp2p Documentation](https://docs.libp2p.io/)

### Implementations
- [Rust CID crate](https://crates.io/crates/cid)
- [Rust Multihash crate](https://crates.io/crates/multihash)
- [Iroh](https://iroh.computer/)
- [IPFS Kubo](https://github.com/ipfs/kubo)
- [Helia (JS IPFS)](https://github.com/ipfs/helia)

### Papers
- [Kademlia: A Peer-to-peer Information System Based on the XOR Metric](https://pdos.csail.mit.edu/~petar/papers/maymounkov-kademlia-lncs.pdf)
- [IPFS - Content Addressed, Versioned, P2P File System](https://ipfs.io/ipfs/QmR7GSQM93Cx5eAg6a6yRzNde1FQv7uL6X1o4k7zrJa3LX/ipfs.draft3.pdf)

---

## Conclusion

Content-addressed data systems provide a powerful foundation for building distributed, verifiable, and permanent data storage. This guide has covered:

1. **Foundations**: Hash functions and the basic concept of content addressing
2. **Middle Level**: Multihash, Merkle trees, and data structures
3. **Intermediate**: CIDs, IPLD, and content routing with CRPs
4. **Advanced**: Distributed systems with libp2p, Kademlia, and Bitswap
5. **Implementation**: Building a complete system in Rust

The `cid-router` project demonstrates these concepts in practice, showing how to build a flexible content routing layer that can work with multiple storage backends. By understanding these fundamentals, you can build your own content-addressed systems or extend existing ones.
