# Error Correction: RaptorQ and Erasure Coding for Distributed Storage

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.ZeroFS/raptorq/`

---

## Table of Contents

1. [Introduction to Error Correction](#introduction-to-error-correction)
2. [Erasure Coding Fundamentals](#erasure-coding-fundamentals)
3. [Reed-Solomon Codes](#reed-solomon-codes)
4. [Fountain Codes](#fountain-codes)
5. [RaptorQ (RFC 6330)](#raptorq-rfc-6330)
6. [Implementation Details](#implementation-details)
7. [Applications in Distributed Storage](#applications-in-distributed-storage)
8. [Performance Benchmarks](#performance-benchmarks)
9. [Code Examples](#code-examples)

---

## Introduction to Error Correction

### The Problem

Data stored or transmitted across unreliable channels faces:

- **Bit errors**: Cosmic rays, disk rot, network corruption
- **Packet loss**: Network congestion, router failures
- **Node failures**: Disk crashes, datacenter outages
- **Correlated failures**: Power outages, natural disasters

### Solutions

| Approach | Redundancy | Recovery | Overhead |
|----------|------------|----------|----------|
| **Replication** | Full copies | Simple | 200-500% |
| **RAID** | Parity | Single/multi-disk | 10-50% |
| **Erasure Coding** | Encoded symbols | Mathematical reconstruction | 10-60% |
| **Checksums** | Detection only | None | <1% |

### Why Erasure Coding?

**Replication (3x):**
```
Data: [A]
Store: [A] [A] [A]  ← 300% storage

Lose 2 copies: Still have 1 ✓
Lose all 3: Data lost ✗
```

**Erasure Coding (4+2):**
```
Data: [A] [B] [C] [D]
Encode: [A] [B] [C] [D] [E] [F]  ← 150% storage
        where E, F are parity

Lose any 2: Can reconstruct ✓
Lose 3+: Data lost ✗
```

---

## Erasure Coding Fundamentals

### Basic Concepts

**Erasure coding** transforms k data symbols into n encoded symbols (n > k):

```
┌─────────────────────────────────────────┐
│         Erasure Coding Process           │
├─────────────────────────────────────────┤
│                                         │
│  Data:  [D0] [D1] [D2] ... [Dk-1]      │
│          │                              │
│          │ Encode (n, k)                │
│          ▼                              │
│  Encoded: [E0] [E1] ... [En-1]         │
│                                         │
│  Property: Any k of n symbols can      │
│            reconstruct original data    │
│                                         │
└─────────────────────────────────────────┘
```

### Key Parameters

| Parameter | Symbol | Description |
|-----------|--------|-------------|
| **Data symbols** | k | Number of original data units |
| **Encoded symbols** | n | Total symbols after encoding |
| **Parity symbols** | n - k | Redundancy symbols |
| **Code rate** | k/n | Efficiency (higher = less overhead) |
| **Failure tolerance** | n - k | Max symbol losses |

### Example: (6, 4) Code

```
Data: 4 symbols (D0, D1, D2, D3)
Encoded: 6 symbols (E0, E1, E2, E3, E4, E5)

Encoding:
E0 = D0
E1 = D1
E2 = D2
E3 = D3
E4 = D0 ⊕ D1 ⊕ D2 ⊕ D3  (XOR parity)
E5 = D0 ⊕ 2·D1 ⊕ 3·D2 ⊕ 4·D3  (Reed-Solomon parity)

Recovery:
- Lose E0: Recover from E4, E5, E1, E2, E3
- Lose E4, E5: Still have all data (E0-E3)
- Lose any 2: Solvable system of equations
```

---

## Reed-Solomon Codes

### Overview

**Reed-Solomon (RS) codes** are optimal erasure codes:

- **Maximum Distance Separable (MDS)**: Any k symbols recover data
- **Symbol-based**: Operate on bytes/symbols, not bits
- **Widely used**: CDs, DVDs, QR codes, RAID 6, space communication

### Galois Field Arithmetic

RS codes operate over **Galois Fields (GF)**:

```
GF(2^8) = Finite field with 256 elements (bytes)

Operations:
- Addition: XOR (a + b = a ⊕ b)
- Multiplication: Polynomial multiplication mod irreducible polynomial
- Division: Multiplication by inverse

Properties:
- Closed: Result always in field
- Associative, commutative, distributive
- Every non-zero element has multiplicative inverse
```

### GF(2^8) Implementation

```rust
// GF(2^8) with primitive polynomial x^8 + x^4 + x^3 + x + 1 (0x11D)
const GF_EXP: [u8; 512] = [/* precomputed exponentials */];
const GF_LOG: [u8; 256] = [/* precomputed logarithms */];

fn gf_mul(a: u8, b: u8) -> u8 {
    if a == 0 || b == 0 {
        return 0;
    }
    GF_EXP[(GF_LOG[a as usize] as usize + GF_LOG[b as usize] as usize) % 255]
}

fn gf_div(a: u8, b: u8) -> u8 {
    if b == 0 {
        panic!("Division by zero");
    }
    if a == 0 {
        return 0;
    }
    GF_EXP[(GF_LOG[a as usize] as usize + 255 - GF_LOG[b as usize] as usize) % 255]
}

fn gf_inv(a: u8) -> u8 {
    gf_div(1, a)
}
```

### Encoding Process

```rust
// Simplified Reed-Solomon encoding
fn rs_encode(data: &[u8], num_parity: usize) -> Vec<u8> {
    let k = data.len();
    let mut encoded = data.to_vec();

    // Generate Vandermonde matrix
    // [ 1   1   1   ...  1   ]
    // [ 1   2   3   ...  n   ]
    // [ 1  2^2 3^2 ...  n^2  ]
    // [...                  ]

    for i in 0..num_parity {
        let mut parity = 0u8;
        for (j, &datum) in data.iter().enumerate() {
            // Evaluate polynomial at point (k + i + 1)
            let x = (k + i + 1) as u8;
            parity ^= gf_mul(datum, gf_pow(x, j));
        }
        encoded.push(parity);
    }

    encoded
}
```

### Decoding (Recovery)

```rust
fn rs_decode(received: &[Option<u8>], k: usize) -> Result<Vec<u8>> {
    // 1. Build erasure polynomial (mark missing positions)
    let erasure_locs: Vec<usize> = received.iter()
        .enumerate()
        .filter(|(_, v)| v.is_none())
        .map(|(i, _)| i)
        .collect();

    // 2. Compute syndromes (check if errors exist)
    let syndromes = compute_syndromes(received);

    // 3. Solve for error locations (Berlekamp-Massey)
    let error_locs = berlekamp_massey(&syndromes)?;

    // 4. Solve for error values (Forney algorithm)
    let error_vals = forney_algorithm(&syndromes, &error_locs)?;

    // 5. Correct errors
    let mut corrected = received.iter()
        .map(|v| v.unwrap_or(0))
        .collect::<Vec<_>>();

    for (loc, val) in error_locs.iter().zip(error_vals.iter()) {
        corrected[*loc] ^= *val;
    }

    Ok(corrected[..k].to_vec())
}
```

### Limitations of Reed-Solomon

| Issue | Impact |
|-------|--------|
| **O(n²) encoding** | Slow for large n |
| **O(n³) decoding** | Very slow for large n |
| **Fixed rate** | Must choose (n, k) upfront |
| **All-or-nothing** | Need exactly k symbols |

---

## Fountain Codes

### The Innovation

**Fountain codes** (rateless erasure codes) overcome RS limitations:

```
Fountain Code Properties:
1. Generate unlimited encoded symbols from k data symbols
2. Recover data from ANY k(1 + ε) received symbols
3. Encoding/decoding complexity: O(k log(1/ε))

Analogy:
- Data = Water in fountain
- Encoded symbols = Water droplets
- Collect enough droplets → Reconstruct the water
```

### LT Codes (Luby Transform)

The first practical fountain code:

```
Encoding:
1. Choose random degree d from distribution
2. Select d data symbols uniformly
3. XOR them to create encoded symbol
4. Transmit (symbol, seed) pair

Decoding:
1. Find degree-1 encoded symbols
2. Recover data symbol
3. XOR recovered symbol into other encoded symbols
4. Repeat until all data recovered
```

### Robust Soliton Distribution

LT codes use a special degree distribution:

```
μ(d) = Ideal Soliton + Robustness addition

Ideal Soliton:
μ(1) = 1/k
μ(d) = 1/(d(d-1)) for d > 1

Robustness adds spikes at:
- d = 1
- d ≈ k/R where R = k · ln(k/δ)

Result: Ensure degree-1 symbols throughout decoding
```

### Limitations of LT Codes

- **Overhead**: Need k(1 + ε) symbols, ε can be 5-10%
- **Decoding failures**: Small but non-zero probability
- **No precode**: Raw data not directly transmitted

---

## RaptorQ (RFC 6330)

### Overview

**RaptorQ** is the most advanced fountain code:

- **RFC 6330**: IETF standard (2011)
- **Rateless**: Generate unlimited symbols
- **Efficient**: Linear time encoding/decoding
- **Reliable**: Reconstruction probability = 1 - 1/256^(h+1)

### Two-Layer Structure

```
┌─────────────────────────────────────────┐
│         RaptorQ Architecture            │
├─────────────────────────────────────────┤
│                                         │
│  Data Symbols                           │
│       │                                 │
│       ▼                                 │
│  ┌─────────────────────────┐           │
│  │  Precode (LDPC + HDPC)  │           │
│  │  - LDPC: Low-density    │           │
│  │  - HDPC: High-density   │           │
│  └───────────┬─────────────┘           │
│              │                          │
│              ▼                          │
│  Intermediate Symbols                   │
│       │                                 │
│       ▼                                 │
│  ┌─────────────────────────┐           │
│  │  LT Code                │           │
│  │  - Sparse graph         │           │
│  │  - Degree distribution  │           │
│  └───────────┬─────────────┘           │
│              │                          │
│              ▼                          │
│  Encoded Symbols                        │
│                                         │
└─────────────────────────────────────────┘
```

### Precode Phase

**LDPC (Low-Density Parity-Check):**
```
Systematic: Original data symbols included
Parity: Generated from data

LDPC Graph:
D0 ──┬── P0
D1 ──┼──┬── P1
D2 ──┼──┼──┬── P2
D3 ──┴──┼──┼── P3
        └──┼── P4
           └── P5

Each parity connects to few data symbols (low density)
```

**HDPC (High-Density Parity-Check):**
```
H0 = D0 ⊕ D1 ⊕ D2 ⊕ D3 ⊕ ... (all data)
H1 = D0 ⊕ 2·D1 ⊕ 3·D2 ⊕ 4·D3 ⊕ ... (weighted)
...

Provides additional protection against failures
```

### LT Encoding Phase

```rust
// Simplified RaptorQ LT encoding
fn lt_encode(
    intermediate_symbols: &[[u8; SYMBOL_SIZE]],
    encoding_symbol_id: u32,
) -> [u8; SYMBOL_SIZE] {
    // Determine degree from distribution
    let degree = get_degree(encoding_symbol_id, intermediate_symbols.len());

    // Generate pseudo-random neighbors
    let neighbors = generate_neighbors(
        encoding_symbol_id,
        degree,
        intermediate_symbols.len(),
    );

    // XOR selected intermediate symbols
    let mut result = [0u8; SYMBOL_SIZE];
    for neighbor_idx in neighbors {
        for (i, &byte) in intermediate_symbols[neighbor_idx].iter().enumerate() {
            result[i] ^= byte;
        }
    }

    result
}
```

### Systematic Encoding

RaptorQ is **systematic**: original data symbols are transmitted first.

```
Encoding Flow:
1. Data symbols → Precode → Intermediate symbols
2. Intermediate symbols[0..k-1] = Data symbols (systematic)
3. Generate repair symbols using LT encoding
4. Transmit: [Data] [Repair 1] [Repair 2] ...

Decoding:
- Receive any k(1 + ε) symbols
- Build constraint matrix
- Solve system of equations
- Recover all intermediate symbols
- Extract original data
```

### Decoding Algorithm

**Gaussian Elimination with Peeling:**

```rust
fn raptorq_decode(
    received_symbols: &[EncodedSymbol],
    k: usize,
) -> Result<Vec<Vec<u8>>> {
    // 1. Build constraint matrix
    let mut matrix = build_constraint_matrix(received_symbols, k);

    // 2. Peeling decoder (belief propagation)
    // Find rows with degree 1, recover symbol, substitute
    while !matrix.is_fully_decoded() {
        if let Some(degree_one_row) = matrix.find_degree_one() {
            let symbol_idx = degree_one_row.unknown_symbol();
            let value = degree_one_row.value();

            // Recover symbol
            intermediate_symbols[symbol_idx] = value;

            // Substitute into other equations
            matrix.substitute(symbol_idx, value);
        } else {
            // Need Gaussian elimination for remaining
            break;
        }
    }

    // 3. Inactivation decoding (if needed)
    // Select variables to "inactivate", solve reduced system
    if !matrix.is_fully_decoded() {
        gaussian_elimination(&mut matrix)?;
    }

    // 4. Extract original data from intermediate symbols
    Ok(intermediate_symbols[..k].to_vec())
}
```

### RFC 6330 Parameters

| Parameter | Value | Description |
|-----------|-------|-------------|
| **Max source symbols (K)** | 56,403 | Per source block |
| **Max encoded symbols** | 16,777,216 | Per source block |
| **Symbol size (T)** | 8 - 65,536 bytes | Configurable |
| **Max object size** | ~9 trillion bytes | K × max T |

### Recovery Probability

```
Recovery probability after receiving K + h symbols:
P(recovery) = 1 - 1/256^(h+1)

Additional symbols (h) | Recovery Probability
───────────────────────┼─────────────────────
0                      | 99.6%
1                      | 99.9996%
2                      | 99.9999996%
3                      | 99.9999999996%

With just 2 extra symbols: Essentially guaranteed recovery
```

---

## Implementation Details

### Object Transmission Information

```rust
// From raptorq/src/base.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectTransmissionInformation {
    transfer_length: u64,      // Total data size
    symbol_size: u16,          // Size of each symbol
    data_replacement_size: u16,
    symbol_alignment_size: u8,
    encoder_symbol_size: u16,
}

impl ObjectTransmissionInformation {
    pub fn generate_encoding_parameters(
        transfer_length: u64,
        max_packet_size: u16,
        decoder_memory_requirement: u64,
    ) -> Self {
        // Calculate optimal symbol size and block count
        // Balance between:
        // - Memory usage (fewer blocks = less memory)
        // - Parallelism (more blocks = more parallelism)
        // - Overhead (larger symbols = less overhead)
    }
}
```

### Block Partitioning

```rust
// Partition data into source blocks
pub fn partition(
    kt: u64,  // Total symbols needed
    max_blocks: u32,
) -> (u32, u32, u32, u32) {
    // Returns (kl, ks, zl, zs)
    // kl: Large block symbol count
    // ks: Small block symbol count
    // zl: Number of large blocks
    // zs: Number of small blocks

    // Ensure blocks are roughly equal size
    // Some blocks may have one more symbol than others
}

// Calculate block offsets
pub fn calculate_block_offsets(
    data: &[u8],
    config: &ObjectTransmissionInformation,
) -> Vec<(usize, usize)> {
    let kt = int_div_ceil(config.transfer_length(), config.symbol_size() as u64);
    let (kl, ks, zl, zs) = partition(kt, config.source_blocks());

    let mut blocks = vec![];
    let mut data_index = 0;

    // Large blocks
    for _ in 0..zl {
        let offset = kl as usize * config.symbol_size() as usize;
        blocks.push((data_index, data_index + offset));
        data_index += offset;
    }

    // Small blocks
    for _ in 0..zs {
        let offset = ks as usize * config.symbol_size() as usize;
        blocks.push((data_index, data_index + offset));
        data_index += offset;
    }

    blocks
}
```

### Intermediate Symbol Generation

```rust
// From raptorq/src/encoder.rs
fn generate_intermediate_symbols(
    source_symbols: &[Vec<u8>],
) -> Vec<Vec<u8>> {
    let k = source_symbols.len();

    // 1. Pre-allocate intermediate symbols
    // S = number of LDPC symbols
    // H = number of HDPC symbols
    // L = K + S + H (total intermediate)
    let s = calculate_num_ldpc_symbols(k);
    let h = calculate_num_hdpc_symbols(k);
    let l = k + s + h;

    let mut intermediate = vec![vec![0u8; SYMBOL_SIZE]; l];

    // 2. Copy source symbols
    for (i, source) in source_symbols.iter().enumerate() {
        intermediate[i].clone_from_slice(source);
    }

    // 3. Generate LDPC parity symbols
    for i in 0..s {
        // Each LDPC symbol depends on a few source symbols
        let neighbors = get_ldpc_neighbors(i, k);
        for &neighbor in &neighbors {
            xor_symbols(&mut intermediate[k + i], &intermediate[neighbor]);
        }
    }

    // 4. Generate HDPC parity symbols
    for i in 0..h {
        // HDPC uses matrix multiplication over GF(256)
        let hdpc_row = get_hdpc_row(i, k, s);
        multiply_and_accumulate(&mut intermediate[k + s + i], &intermediate, &hdpc_row);
    }

    // 5. Apply constrained decoding inverse
    // (ensures systematic property)
    apply_precode_inverse(&mut intermediate);

    intermediate
}
```

### Constraint Matrix

```rust
// Sparse constraint matrix for decoding
struct SparseConstraintMatrix {
    // Each row: (coefficient, column) pairs
    rows: Vec<BTreeMap<usize, u8>>,
    // Right-hand side values
    rhs: Vec<Vec<u8>>,
}

impl SparseConstraintMatrix {
    // Gaussian elimination optimized for sparse matrices
    fn decode(&mut self) -> Result<Vec<Vec<u8>>> {
        let n = self.rows.len();

        for col in 0..n {
            // Find pivot
            let pivot_row = (col..n)
                .find(|&r| self.rows[r].contains_key(&col))
                .ok_or("Matrix singular")?;

            // Swap rows
            self.rows.swap(col, pivot_row);
            self.rhs.swap(col, pivot_row);

            // Eliminate column from other rows
            for row in 0..n {
                if row != col && self.rows[row].contains_key(&col) {
                    let factor = self.rows[row][&col];
                    let pivot_factor = self.rows[col][&col];
                    let multiplier = gf_div(factor, pivot_factor);

                    // row = row - multiplier * pivot
                    self.add_scaled_row(col, row, multiplier);
                }
            }
        }

        // Extract solution
        let mut solution = vec![vec![0u8; SYMBOL_SIZE]; n];
        for (i, row) in self.rows.iter().enumerate() {
            let col = row.keys().next().unwrap();
            let coef = row[col];
            solution[*col] = gf_scalar_mul(&self.rhs[i], gf_inv(coef));
        }

        Ok(solution)
    }
}
```

---

## Applications in Distributed Storage

### 1. Multi-Region Storage

```
Use Case: Store data across 3 AWS regions

Without erasure coding:
- Replicate 3x
- Cost: 300% of data size

With RaptorQ (6, 4):
- Encode 4 data → 6 symbols
- Store 2 symbols per region
- Tolerate: Lose 1 entire region + 1 symbol
- Cost: 150% of data size
```

### 2. Distributed File Systems (ZeroFS)

```
ZeroFS with RaptorQ:

File → Split into chunks → RaptorQ encode → Store to S3

Benefits:
- Tolerate S3 region failures
- Reduce storage costs vs replication
- Reconstruct from any subset of regions

Architecture:
┌─────────────────────────────────────────┐
│  File (1 MB)                            │
│       │                                 │
│       ▼                                 │
│  RaptorQ Encoder (k=4, overhead=2)     │
│       │                                 │
│       ▼                                 │
│  [S0] [S1] [S2] [S3] [S4] [S5]        │
│   │    │    │    │    │    │           │
│   ▼    ▼    ▼    ▼    ▼    ▼           │
│  us   us   eu   eu   ap   ap           │
│  east west west south east south        │
│                                         │
│  Recovery: Any 4 symbols → Full file   │
└─────────────────────────────────────────┘
```

### 3. Content Delivery

```
Video Streaming with Fountain Codes:

Server:
- Encode video with RaptorQ
- Stream unlimited encoded packets

Clients:
- Collect any k(1+ε) packets
- Decode and play

Benefits:
- No retransmission needed
- Clients can join/leave freely
- Server doesn't track client state
```

### 4. Peer-to-Peer Networks

```
BitTorrent with Erasure Coding:

Traditional BitTorrent:
- Need specific pieces (rarest first)
- "Last piece problem": rare pieces bottleneck

With Fountain Codes:
- All encoded pieces equally valuable
- No "last piece problem"
- Faster completion time
```

### 5. Archival Storage

```
Long-term Data Preservation:

Challenge: Media degradation over decades

Solution: RaptorQ + periodic refresh

Process:
1. Encode archival data with low code rate (e.g., 10+6)
2. Store across multiple media/archives
3. Periodically read and verify
4. If any symbols corrupted, regenerate from survivors
5. No need to recover full data for refresh
```

---

## Performance Benchmarks

### RaptorQ Performance (from raptorq README)

**Ryzen 9 5900X @ 3.70GHz:**

| Symbol Count | Encoding Throughput | Decoding Throughput |
|--------------|--------------------|--------------------|
| 10 | 4.7 Gbit/s | 3.2 Gbit/s |
| 100 | 4.7 Gbit/s | 3.2 Gbit/s |
| 1,000 | 4.7 Gbit/s | 3.3 Gbit/s |
| 10,000 | 3.4 Gbit/s | 2.6 Gbit/s |
| 50,000 | 2.0 Gbit/s | 1.6 Gbit/s |

**With pre-built encoding plan:**

| Symbol Count | Encoding Throughput |
|--------------|--------------------|
| 10 | 8.6 Gbit/s |
| 100 | 12.2 Gbit/s |
| 1,000 | 10.9 Gbit/s |
| 10,000 | 7.1 Gbit/s |

**Intel i5-6600K @ 3.50GHz:**

| Symbol Count | Encoding Throughput | Decoding Throughput |
|--------------|--------------------|--------------------|
| 10 | 2.4 Gbit/s | 1.7 Gbit/s |
| 100 | 2.6 Gbit/s | 2.1 Gbit/s |
| 1,000 | 2.7 Gbit/s | 2.3 Gbit/s |
| 10,000 | 2.0 Gbit/s | 1.6 Gbit/s |

**Raspberry Pi 3 B+ (Cortex-A53 @ 1.4GHz):**

| Symbol Count | Encoding Throughput | Decoding Throughput |
|--------------|--------------------|--------------------|
| 10 | 202 Mbit/s | 156 Mbit/s |
| 100 | 258 Mbit/s | 207 Mbit/s |
| 1,000 | 221 Mbit/s | 183 Mbit/s |
| 10,000 | 155 Mbit/s | 130 Mbit/s |

### Comparison with Reed-Solomon

| Metric | Reed-Solomon | RaptorQ |
|--------|--------------|---------|
| Encoding complexity | O(n²) | O(n log n) |
| Decoding complexity | O(n³) | O(n log n) |
| Symbol loss tolerance | Exactly n-k | Any (1+ε)k |
| Rate flexibility | Fixed | Rateless |
| Implementation | Complex | Simpler |

---

## Code Examples

### Basic RaptorQ Usage

```rust
use raptorq::{Encoder, Decoder, EncoderBuilder};

fn main() {
    // Data to encode
    let data = b"Hello, RaptorQ! This is a test of erasure coding.";

    // Create encoder with default parameters
    let encoder = Encoder::with_defaults(data, 1280);  // 1280 byte MTU

    // Generate encoded symbols
    let mut symbols = Vec::new();
    let encoding_packet_info = encoder.get_config();

    // Generate systematic symbols (original data)
    for i in 0..encoding_packet_info.source_symbols() {
        let packet = encoder.get_sourceblock(i, 0, 1280);
        symbols.push(packet);
    }

    // Generate repair symbols
    for i in 0..10 {
        let packet = encoder.get_repairblock(i, 0, 1280);
        symbols.push(packet);
    }

    // Simulate packet loss (remove some symbols)
    symbols.remove(2);  // Lose source symbol
    symbols.remove(5);  // Lose another
    symbols.remove(0);  // Lose first

    // Decode
    let mut decoder = Decoder::new(encoding_packet_info);
    for symbol in symbols {
        decoder.add(symbol);
    }

    let result = decoder.wait();
    assert_eq!(result.unwrap(), data);
}
```

### Distributed Storage Example

```rust
use raptorq::{Encoder, Decoder, SourceBlockEncoder};

struct DistributedStorage {
    regions: Vec<String>,
    k: usize,  // Data symbols
    n: usize,  // Total symbols
}

impl DistributedStorage {
    pub fn new(regions: Vec<String>, k: usize, n: usize) -> Self {
        assert!(n > k, "Need more total symbols than data");
        Self { regions, k, n }
    }

    pub fn store(&self, data: &[u8]) -> Result<()> {
        // Encode data
        let encoder = Encoder::with_defaults(data, 4096);

        // Get all encoded symbols
        let symbols = self.generate_symbols(&encoder);

        // Distribute across regions
        let symbols_per_region = self.n / self.regions.len();
        for (region_idx, region) in self.regions.iter().enumerate() {
            let start = region_idx * symbols_per_region;
            let end = start + symbols_per_region;
            let region_symbols = &symbols[start..end];

            // Store to region (e.g., S3)
            self.store_to_region(region, region_symbols)?;
        }

        Ok(())
    }

    pub fn retrieve(&self) -> Result<Vec<u8>> {
        let mut decoder = Decoder::new(self.get_config());
        let mut symbols_needed = self.k;
        let mut symbols_received = 0;

        // Try to retrieve from regions
        for region in &self.regions {
            if symbols_received >= symbols_needed {
                break;
            }

            match self.retrieve_from_region(region) {
                Ok(symbols) => {
                    for symbol in symbols {
                        decoder.add(symbol);
                        symbols_received += 1;
                        if symbols_received >= symbols_needed {
                            break;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Region {} unavailable: {}", region, e);
                    // Need more symbols from other regions
                    symbols_needed += self.k / self.regions.len();
                }
            }
        }

        decoder.wait().ok_or_else(|| {
            anyhow::anyhow!("Not enough symbols to reconstruct")
        })
    }

    fn generate_symbols(&self, encoder: &Encoder) -> Vec<EncodingPacket> {
        let mut symbols = Vec::new();

        // Systematic symbols
        for i in 0..self.k {
            let packet = encoder.get_sourceblock(i as u32, 0, 4096);
            symbols.push(packet);
        }

        // Repair symbols
        for i in 0..(self.n - self.k) {
            let packet = encoder.get_repairblock(i as u32, 0, 4096);
            symbols.push(packet);
        }

        symbols
    }
}

// Usage
fn main() -> Result<()> {
    let storage = DistributedStorage::new(
        vec!["us-east".into(), "eu-west".into(), "ap-south".into()],
        4,  // 4 data symbols
        6,  // 6 total symbols (can lose 2)
    );

    let data = b"This is important data that needs high durability!";

    // Store data across 3 regions
    storage.store(data)?;

    // Later: retrieve data (even if 1 region is down)
    let retrieved = storage.retrieve()?;
    assert_eq!(retrieved, data);

    Ok(())
}
```

### Custom Encoding Plan

```rust
use raptorq::{Encoder, SourceBlockEncodingPlan};

// Pre-build encoding plan for repeated use
// (Same symbol count, different data)
let plan = SourceBlockEncodingPlan::generate(1000);

// Use plan for multiple encodings
for data_chunk in data_chunks {
    let encoder = Encoder::with_plan(
        data_chunk,
        4096,
        &plan,  // Reuse plan
    );

    // Encode faster than generating new plan each time
    let symbols = encode_all_symbols(&encoder);
    store_symbols(symbols);
}
```

---

## Summary

### Key Takeaways

1. **Erasure coding** provides better efficiency than replication:
   - Replication: 200-500% overhead
   - Erasure coding: 10-60% overhead

2. **Reed-Solomon codes** are optimal but slow:
   - O(n²) encoding, O(n³) decoding
   - Need exactly k symbols

3. **Fountain codes** are rateless and efficient:
   - Generate unlimited symbols
   - Recover from any k(1+ε) symbols
   - O(n log n) encoding/decoding

4. **RaptorQ (RFC 6330)** is the state of the art:
   - Two-layer: Precode (LDPC+HDPC) + LT code
   - Systematic: Original data included
   - Recovery probability: 1 - 1/256^(h+1)
   - Throughput: 2-12 Gbit/s depending on hardware

5. **Applications in distributed storage**:
   - Multi-region durability
   - Reduced storage costs
   - No single point of failure
   - Efficient reconstruction

### Further Reading

- [RFC 6330: RaptorQ Specification](https://datatracker.ietf.org/doc/html/rfc6330)
- [RaptorQ Technical Overview (Qualcomm)](https://www.qualcomm.com/media/documents/files/raptorq-technical-overview.pdf)
- [raptorq crate documentation](https://docs.rs/raptorq)
- [Luby Transform Codes Original Paper](https://ieeexplore.ieee.org/document/1021311)
