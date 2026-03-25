# Merkle Trees: Implementation and Applications in Distributed Systems

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.ZeroFS/merkle/, ct-merkle/`

---

## Table of Contents

1. [Introduction to Merkle Trees](#introduction-to-merkle-trees)
2. [Merkle Tree Fundamentals](#merkle-tree-fundamentals)
3. [RFC 6962 (Certificate Transparency) Merkle Trees](#rfc-6962-certificate-transparency-merkle-trees)
4. [Merkle Proofs](#merkle-proofs)
5. [Compact Certificates (C2SP)](#compact-certificates-c2sp)
6. [Implementation Details](#implementation-details)
7. [Applications in Distributed Systems](#applications-in-distributed-systems)
8. [Code Examples](#code-examples)

---

## Introduction to Merkle Trees

### What is a Merkle Tree?

A **Merkle tree** (or hash tree) is a tree data structure where:

- **Leaf nodes** contain cryptographic hashes of data blocks
- **Non-leaf nodes** contain hashes of their child nodes
- **Root node** provides a single cryptographic digest of all data

```
                    Root Hash
                   /         \
              Hash(A,B)     Hash(C,D)
              /     \       /     \
           Hash(A) Hash(B) Hash(C) Hash(D)
           /         |       |         \
         Data A   Data B  Data C     Data D
```

### Why Merkle Trees?

| Property | Benefit |
|----------|---------|
| **Efficient Verification** | Prove data inclusion in O(log n) |
| **Small Footprint** | Root hash is fixed size (32 bytes for SHA-256) |
| **Tamper Evidence** | Any change invalidates the root |
| **Incremental Updates** | Update one leaf, recalculate path to root |
| **Parallel Computation** | Hash subtrees independently |

### History

- **1979**: Ralph Merkle patents the concept
- **2000s**: Used in peer-to-peer networks (BitTorrent)
- **2009**: Bitcoin uses Merkle trees for transaction verification
- **2013**: Certificate Transparency uses Merkle trees for audit logs
- **2020s**: Static CT proposes new format for Merkle tree storage

---

## Merkle Tree Fundamentals

### Binary Merkle Trees

The most common form is a **binary Merkle tree**:

```
Construction (bottom-up):
1. Hash each data block to create leaf nodes
2. Pair adjacent nodes, concatenate, hash to create parent
3. Repeat until single root node

Example with 4 leaves:
Leaf 0: H(D0)
Leaf 1: H(D1)
Leaf 2: H(D2)
Leaf 3: H(D3)

Node 0-1: H(H(D0) || H(D1))
Node 2-3: H(H(D2) || H(D3))

Root: H(Node 0-1 || Node 2-3)
```

### Handling Non-Power-of-Two Leaves

When the number of leaves is not a power of 2:

**RFC 6962 Approach:**
```
5 leaves:
         Root
        /    \
      N0      N1
     /  \    /  \
   L0   L1 L2   N2
               /  \
             L3   L4

Promote nodes when no sibling exists
```

### Tree Size and Depth

For n leaves:
- **Tree depth**: ⌈log₂(n)⌉
- **Total nodes**: 2n - 1 (for perfect binary tree)
- **Proof size**: O(log n) hashes

```
Leaves | Depth | Proof Size (SHA-256)
───────┼───────┼─────────────────────
   4   |   2   | 64 bytes (2 hashes)
  16   |   4   | 128 bytes (4 hashes)
 256   |   8   | 256 bytes (8 hashes)
65536  |  16   | 512 bytes (16 hashes)
```

---

## RFC 6962 (Certificate Transparency) Merkle Trees

### Overview

RFC 6962 defines Merkle trees for **Certificate Transparency (CT)** logs:

- **Leaves**: Certificate entries (with prefix byte)
- **Internal nodes**: Concatenation with prefix byte
- **Root**: Signed Tree Head (STH)

### Leaf Hash Computation

```
leaf_hash = Hash(0x00 || leaf_data)
           ^^^^
           Prefix byte distinguishes leaf from node
```

### Internal Node Hash Computation

```
node_hash = Hash(0x01 || left_child_hash || right_child_hash)
           ^^^^
           Prefix byte distinguishes node from leaf
```

### Why Prefix Bytes?

**Prevents second preimage attacks:**

```
Without prefix:
Tree 1:          Tree 2:
   Root             Root
  /    \           /    \
H(A)  H(B)       H(A||B)  X

If H(A||B) = Hash(H(A) || H(B)), both trees have same root!

With prefix:
Leaf hash:  H(0x00 || data)
Node hash:  H(0x01 || left || right)

Now H(0x00 || A||B) ≠ H(0x01 || H(0x00||A) || H(0x00||B))
```

### Signed Tree Head (STH)

```rust
struct SignedTreeHead {
    version: u8,           // STH version
    signature_type: u8,    // CertificateTimestampType
    timestamp: u64,        // Milliseconds since epoch
    tree_size: u64,        // Number of entries
    sha256_root_hash: [u8; 32],  // Merkle root
    extensions: Vec<u8>,   // Optional extensions
    signature: DigitallySigned,  // Log's signature
}
```

### STH Lifecycle

```
1. Log accepts certificates
2. Log periodically publishes STH (e.g., every 1000 certs)
3. STH is signed and distributed
4. Clients verify certificates against STH
5. Monitors check STH consistency over time
```

---

## Merkle Proofs

### Inclusion Proofs

Prove a leaf is in the tree:

```
Tree:
         Root (R)
        /      \
      K1        K2
     /  \      /  \
   L0   L1   L2   L3

To prove L1 is in tree:
Proof = [L0, K2]

Verification:
1. Hash L1 to get H(L1)
2. Hash with sibling: H(H(L0) || H(L1)) = K1
3. Hash with parent's sibling: H(K1 || K2) = R
4. Compare with known root R
```

### RFC 6962 Inclusion Proof Format

```rust
struct InclusionProof {
    leaf_index: u64,      // Index of leaf in tree
    tree_size: u64,       // Size of tree at time of proof
    hashes: Vec<[u8; 32]>, // Hashes along path to root
}

fn verify_inclusion(
    leaf_hash: &[u8; 32],
    proof: &InclusionProof,
    root_hash: &[u8; 32],
) -> bool {
    let mut current_hash = *leaf_hash;
    let mut index = proof.leaf_index;
    let mut size = proof.tree_size;

    for (i, sibling) in proof.hashes.iter().enumerate() {
        if index & 1 == 0 {
            // Current is left sibling
            current_hash = hash_node(&current_hash, sibling);
        } else {
            // Current is right sibling
            current_hash = hash_node(sibling, &current_hash);
        }
        index >>= 1;
        size >>= 1;
    }

    current_hash == *root_hash
}
```

### Consistency Proofs

Prove two trees are consistent (later tree extends earlier):

```
Tree 1 (size 3):     Tree 2 (size 5):
      R1                   R2
     /  \                 /  \
   N1    N2            N3     N4
  /  \   |            /  \   /  \
 L0  L1  L2         L0  L1 L2  N5
                                   / \
                                 L3  L4

Consistency proof shows:
- Tree 1's root can be computed from Tree 2's nodes
- Tree 2 contains all of Tree 1's leaves in same order
```

### Consistency Proof Algorithm

```rust
fn compute_consistency_proof(
    old_size: u64,
    new_size: u64,
    get_hash: impl Fn(u64, u64) -> [u8; 32],
) -> Vec<[u8; 32]> {
    let mut proof = Vec::new();
    let mut left = 0u64;
    let mut right = old_size;
    let mut old_root = None;

    // Find subtrees that cover old tree
    while right > 0 {
        let mut k = largest_power_of_two_less_than(right);

        if left + k == old_size {
            // This subtree's root is in old tree
            old_root = Some(get_hash(left, k));
        } else if left < old_size {
            // This subtree overlaps with old tree
            proof.push(get_hash(left + k, right - k));
        } else {
            // This subtree is entirely after old tree
            proof.push(get_hash(left, k));
        }

        left += k;
        right -= k;
    }

    if let Some(root) = old_root {
        proof.push(root);
    }

    proof
}
```

---

## Compact Certificates (C2SP)

### The Problem with RFC 6962

Traditional CT logs have scalability issues:

1. **O(n log n) storage**: Each leaf requires log n nodes
2. **Expensive updates**: Every new leaf updates O(log n) nodes
3. **Versioning overhead**: Track all historical nodes

### Static CT Solution

Static CT (C2SP draft) proposes:

1. **Tile-based storage**: Store Merkle tree as fixed-size tiles
2. **Immutable tiles**: Once written, tiles never change
3. **On-demand proofs**: Generate proofs from tiles as needed

### Tile Structure

```
Tile Format:
┌─────────────────────────────────────┐
│  Tile Header                         │
│  - Level (depth from leaves)        │
│  - Index (position at level)        │
│  - Entry count                      │
├─────────────────────────────────────┤
│  Entries                            │
│  - Hash 1 (32 bytes)                │
│  - Hash 2 (32 bytes)                │
│  - ...                              │
├─────────────────────────────────────┤
│  Optional: Compact Prefix Encoding  │
│  - Store only differing suffix      │
│  - Reduce storage by ~30%           │
└─────────────────────────────────────┘
```

### Tile Addressing

```
Tile path: /{level}/{index}

Level 0 (leaves):
  Tile 0: leaves 0-255
  Tile 1: leaves 256-511
  ...

Level 1:
  Tile 0: nodes 0-127 (hashes of tile 0 pairs)
  Tile 1: nodes 128-255
  ...

Root level:
  Tile 0: single root hash
```

### Benefits

| Aspect | RFC 6962 | Static CT |
|--------|----------|-----------|
| Storage | O(n log n) | O(n) |
| Update cost | O(log n) writes | Amortized O(1) |
| CDN serving | Difficult (dynamic) | Native (static tiles) |
| Proof generation | On-demand | On-demand |

---

## Implementation Details

### Hash Functions

**SHA-256** is the standard for Merkle trees:

```rust
use sha2::{Sha256, Digest};

fn hash_leaf(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(&[0x00]);  // Leaf prefix
    hasher.update(data);
    hasher.finalize().into()
}

fn hash_node(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(&[0x01]);  // Node prefix
    hasher.update(left);
    hasher.update(right);
    hasher.finalize().into()
}
```

### Alternative Hash Functions

| Hash | Size | Speed | Use Case |
|------|------|-------|----------|
| **SHA-256** | 32 bytes | Medium | Standard (CT, Bitcoin) |
| **BLAKE3** | 32 bytes | Fast | High-performance logs |
| **SHA-3** | 32 bytes | Slow | Post-quantum resistance |

### Tree Construction

**Batch Construction (offline):**

```rust
fn build_merkle_tree(leaf_hashes: &[[u8; 32]]) -> [u8; 32] {
    if leaf_hashes.is_empty() {
        return EMPTY_TREE_ROOT;
    }

    let mut current_level = leaf_hashes.to_vec();

    while current_level.len() > 1 {
        let mut next_level = Vec::new();

        for chunk in current_level.chunks(2) {
            let hash = if chunk.len() == 2 {
                hash_node(&chunk[0], &chunk[1])
            } else {
                // Promote single node
                chunk[0]
            };
            next_level.push(hash);
        }

        current_level = next_level;
    }

    current_level[0]
}
```

**Incremental Construction (online):**

```rust
struct IncrementalMerkleTree {
    // Store rightmost nodes at each level
    rightmost: Vec<Option<[u8; 32]>>,
    size: usize,
}

impl IncrementalMerkleTree {
    pub fn new() -> Self {
        Self {
            rightmost: Vec::new(),
            size: 0,
        }
    }

    pub fn push(&mut self, leaf_hash: [u8; 32]) {
        let mut current = leaf_hash;
        let mut index = self.size;

        for level in &mut self.rightmost {
            if index & 1 == 0 {
                // Even index: store as rightmost, stop
                *level = Some(current);
                break;
            } else {
                // Odd index: combine with rightmost, continue up
                if let Some(right) = level.take() {
                    current = hash_node(&right, &current);
                } else {
                    *level = Some(current);
                    break;
                }
            }
            index >>= 1;
        }

        // If we exhausted all levels, add new level
        if self.rightmost.iter().all(|n| n.is_none()) {
            self.rightmost.push(Some(current));
        }

        self.size += 1;
    }

    pub fn root(&self) -> [u8; 32] {
        // Reconstruct root from rightmost nodes
        // (simplified - actual implementation needs more care)
        todo!()
    }
}
```

### Memory Efficiency

**CompactLog Optimization** (from the source exploration):

```rust
// Store only at STH boundaries
struct VersionedMerkleTree {
    // In-memory: current tree state
    current_nodes: HashMap<NodeId, [u8; 32]>,

    // Persistent: versioned nodes at STH boundaries
    // Only O(log n) nodes per STH
    versioned_nodes: BTreeMap<(Version, NodeId), [u8; 32]>,

    sth_interval: u64,  // Publish STH every N certificates
}

// Storage reduction:
// Traditional: O(n log n) versioned nodes
// CompactLog:  O(n log n / k) where k = STH interval
//
// Example: k = 1000 → 1000x storage reduction
```

---

## Applications in Distributed Systems

### 1. Certificate Transparency

**Problem:** CAs can issue certificates without domain owner's knowledge

**Solution:** CT logs maintain public Merkle tree of all certificates

```
Certificate Issuance Flow:
1. CA submits cert to CT log
2. Log returns SCT (Signed Certificate Timestamp)
3. CA includes SCT in certificate
4. Browser verifies SCT against log's STH
5. Monitors audit log for misissued certs
```

**CompactLog** serves both RFC 6962 and Static CT APIs from the same tree.

### 2. Content-Addressed Storage

**IPFS, git, etc.** use Merkle trees for content verification:

```
File Storage:
┌─────────────────────────────────────────┐
│  File → Split into blocks               │
│  Block hashes → Merkle tree             │
│  Root hash = Content identifier (CID)   │
└─────────────────────────────────────────┘

Verification:
- Download blocks from any source
- Verify each block's hash
- Verify Merkle proof against CID
- tampered blocks are detected immediately
```

### 3. Blockchain

**Bitcoin, Ethereum** use Merkle trees:

```
Bitcoin Block:
┌─────────────────────────────────────────┐
│  Block Header                           │
│  - Previous block hash                  │
│  - Merkle root of transactions          │
│  - Timestamp                            │
│  - Nonce                                │
└─────────────────────────────────────────┘

SPV (Simplified Payment Verification):
- Download only block headers (80 bytes each)
- Request Merkle proof for specific transaction
- Verify transaction is in block
```

### 4. Distributed Databases

**Apache Cassandra, Amazon DynamoDB** use Merkle trees for anti-entropy:

```
Replica Synchronization:
1. Each replica computes Merkle tree of data
2. Compare root hashes
3. If different, exchange proofs
4. Identify specific divergent keys
5. Sync only affected data
```

### 5. ZeroFS Applications

ZeroFS can use Merkle trees for:

- **Chunk verification**: Verify 32KB chunks haven't been tampered
- **Checkpoint integrity**: Merkle root of checkpoint state
- **Replication verification**: Verify replicas have identical data

---

## Code Examples

### Complete Merkle Tree Implementation

```rust
use sha2::{Sha256, Digest};

const LEAF_PREFIX: u8 = 0x00;
const NODE_PREFIX: u8 = 0x01;

#[derive(Debug, Clone)]
pub struct MerkleTree {
    leaves: Vec<[u8; 32]>,
    layers: Vec<Vec<[u8; 32]>>,
}

impl MerkleTree {
    pub fn new(data: &[Vec<u8>]) -> Self {
        let leaves: Vec<[u8; 32]> = data.iter()
            .map(|d| hash_leaf(d))
            .collect();

        let mut layers = vec![leaves.clone()];

        // Build tree bottom-up
        let mut current_layer = leaves;
        while current_layer.len() > 1 {
            let mut next_layer = Vec::new();

            for chunk in current_layer.chunks(2) {
                let hash = if chunk.len() == 2 {
                    hash_node(&chunk[0], &chunk[1])
                } else {
                    chunk[0]  // Promote single node
                };
                next_layer.push(hash);
            }

            layers.push(next_layer.clone());
            current_layer = next_layer;
        }

        Self { leaves, layers }
    }

    pub fn root(&self) -> [u8; 32] {
        self.layers.last().and_then(|l| l.first()).copied()
            .expect("Empty tree")
    }

    pub fn get_proof(&self, index: usize) -> MerkleProof {
        let mut hashes = Vec::new();
        let mut current_index = index;

        for layer in &self.layers[:-1] {  // Exclude root layer
            let sibling_index = if current_index % 2 == 0 {
                current_index + 1
            } else {
                current_index - 1
            };

            if sibling_index < layer.len() {
                hashes.push(MerkleProofNode {
                    hash: layer[sibling_index],
                    position: if current_index % 2 == 0 {
                        Position::Right
                    } else {
                        Position::Left
                    },
                });
            }

            current_index /= 2;
        }

        MerkleProof {
            leaf_index: index,
            leaf_hash: self.leaves[index],
            hashes,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MerkleProof {
    pub leaf_index: usize,
    pub leaf_hash: [u8; 32],
    pub hashes: Vec<MerkleProofNode>,
}

#[derive(Debug, Clone)]
pub struct MerkleProofNode {
    pub hash: [u8; 32],
    pub position: Position,
}

#[derive(Debug, Clone, Copy)]
pub enum Position {
    Left,
    Right,
}

impl MerkleProof {
    pub fn verify(&self, root: [u8; 32]) -> bool {
        let mut current = self.leaf_hash;

        for node in &self.hashes {
            current = match node.position {
                Position::Left => hash_node(&node.hash, &current),
                Position::Right => hash_node(&current, &node.hash),
            };
        }

        current == root
    }
}

fn hash_leaf(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(&[LEAF_PREFIX]);
    hasher.update(data);
    hasher.finalize().into()
}

fn hash_node(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(&[NODE_PREFIX]);
    hasher.update(left);
    hasher.update(right);
    hasher.finalize().into()
}

// Usage example
fn main() {
    let data = vec![
        b"Hello".to_vec(),
        b"World".to_vec(),
        b"!".to_vec(),
    ];

    let tree = MerkleTree::new(&data);
    println!("Root: {:?}", tree.root());

    let proof = tree.get_proof(1);  // Proof for "World"
    assert!(proof.verify(tree.root()));
}
```

### Incremental Tree with Streaming

```rust
pub struct StreamingMerkleTree {
    leaves: Vec<[u8; 32]>,
    // Store rightmost node at each level
    rightmost: Vec<[u8; 32]>,
    size: usize,
}

impl StreamingMerkleTree {
    pub fn new() -> Self {
        Self {
            leaves: Vec::new(),
            rightmost: Vec::new(),
            size: 0,
        }
    }

    pub fn push(&mut self, data: Vec<u8>) {
        let leaf_hash = hash_leaf(&data);
        self.leaves.push(leaf_hash);

        let mut current = leaf_hash;
        let mut index = self.size;

        for level in &mut self.rightmost {
            if index & 1 == 0 {
                // Even: store and stop
                *level = current;
                break;
            } else {
                // Odd: combine and continue up
                current = hash_node(level, &current);
            }
            index >>= 1;
        }

        if self.rightmost.len() <= index {
            self.rightmost.push(current);
        }

        self.size += 1;
    }

    pub fn root(&self) -> [u8; 32] {
        if self.rightmost.is_empty() {
            return EMPTY_TREE_ROOT;
        }

        let mut result = self.rightmost[0];
        let mut size = self.size;

        for (level, &node) in self.rightmost.iter().enumerate().skip(1) {
            if size & 1 == 1 {
                result = hash_node(&result, &node);
            }
            size >>= 1;
        }

        result
    }
}
```

---

## Summary

### Key Takeaways

1. **Merkle trees** provide efficient, cryptographically secure verification of data inclusion
2. **RFC 6962** defines Merkle trees for Certificate Transparency with:
   - Prefix bytes to distinguish leaves from nodes
   - Signed Tree Heads for audit
   - Inclusion and consistency proofs
3. **Static CT (C2SP)** improves scalability with:
   - Tile-based storage
   - Immutable tiles for CDN serving
   - O(n) storage instead of O(n log n)
4. **Applications** include:
   - Certificate Transparency
   - Content-addressed storage
   - Blockchain
   - Distributed database anti-entropy
5. **CompactLog** demonstrates dual-API serving (RFC 6962 + Static CT) from a single LSM-tree backend

### Further Reading

- [RFC 6962: Certificate Transparency](https://datatracker.ietf.org/doc/html/rfc6962)
- [C2SP Static CT Specification](https://www.c2sp.org/static-ct)
- [CompactLog Documentation](/home/darkvoid/Boxxed/@formulas/src.rust/src.ZeroFS/compact_log/README.md)
- [Merkle's Original Patent](https://en.wikipedia.org/wiki/Merkle_tree)
