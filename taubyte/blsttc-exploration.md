# Taubyte BLS Threshold Cryptography (blsttc) - Deep-Dive Exploration

**Date:** 2026-03-22
**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Taubyte/blsttc/`

---

## 1. Purpose and Overview

The **blsttc** component implements **BLS (Boneh-Lynn-Shacham) Threshold Cryptography** for Taubyte's decentralized network. BLS threshold cryptography enables distributed key generation and threshold signatures, where multiple parties must collaborate to decrypt or sign data, but no single party has access to the complete key.

### Key Characteristics

- **Implementation:** Rust (using blsttc crate)
- **Cryptographic Scheme:** BLS12-381 pairing-friendly curve
- **Purpose:** Threshold encryption/decryption for secure multi-party computation
- **Integration:** Exposed as WebAssembly module for Taubyte VM

---

## 2. Architecture

### 2.1 Module Structure

```
blsttc/
├── src/
│   └── lib.rs           # Main implementation
├── Cargo.toml           # Rust dependencies
├── build.sh             # Build script
├── artifact.wasm        # Compiled WASM module
├── README.md            # Documentation
└── LICENSE              # License file
```

### 2.2 Dependencies

```toml
[dependencies]
blsttc = "0.1"    # BLS threshold cryptography
bincode = "1.3"   # Binary serialization
rand = "0.8"      # Random number generation
taubyte-sdk = "0.1"  # Taubyte SDK for host functions
```

### 2.3 Design Philosophy

1. **Threshold Security:** Requires t-of-n shares for operations
2. **Non-Interactive:** No communication needed during decryption
3. **Verifiable:** Shares can be verified before combination
4. **Efficient:** BLS signatures are short and fast to verify

---

## 3. Key Types, Interfaces, and APIs

### 3.1 Core Cryptographic Types

The implementation uses types from the `blsttc` crate:

```rust
use blsttc::{
    Ciphertext,      // Encrypted data
    DecryptionShare, // Partial decryption share
    PublicKey,       // Public key
    PublicKeySet,    // Set of public keys for threshold scheme
    PK_SIZE,         // Public key size constant
};
```

### 3.2 Exported Functions

#### encrypt

Encrypts a message using a public key:

```rust
/// Encrypts a message using a public key and thread rng.
/// Returns the id of the Memory View with the encrypted data.
///
/// # Arguments
///
/// * `pk_id` - A UInt32 id referring to the Memory View with the stored public key.
/// * `msg_id` - A UInt32 id referring to the Memory View with the stored message.
#[no_mangle]
pub fn encrypt(pk_id: u32, msg_id: u32) -> u32 {
    // 1. Open public key memory view
    let mut pk_mv = ReadSeekCloser::open(pk_id).unwrap();

    // 2. Read public key bytes
    let mut pk_buffer = [0; PK_SIZE];
    let _ = pk_mv.read(&mut pk_buffer).unwrap();

    // 3. Construct PublicKey from bytes
    let pk = PublicKey::from_bytes(pk_buffer).unwrap();

    // 4. Open message memory view
    let mut msg_mv = ReadSeekCloser::open(msg_id).unwrap();

    // 5. Read message bytes
    let mut msg_buffer: Vec<u8> = Vec::new();
    let _ = msg_mv.read_to_end(&mut msg_buffer).unwrap();

    // 6. Encrypt with RNG
    let mut rng = rand::thread_rng();
    let ct = pk.encrypt_with_rng(&mut rng, msg_buffer);

    // 7. Serialize ciphertext
    let bincode_ct_vec = bincode::serialize(&ct).unwrap();

    // 8. Reverse bytes (endianness fix)
    let beb = reverse_bytes(bincode_ct_vec);

    // 9. Store in memory view and return ID
    Closer::new(&beb, true).unwrap().id
}
```

#### decrypt

Decrypts ciphertext using combined decryption shares:

```rust
/// Decrypts the ciphered text using the recombined decryption shares,
/// and returns the id of the Memory View with the decrypted data.
///
/// # Arguments
///
/// * `public_key_set_id` - Memory View id with the stored public key set.
/// * `shares_id` - Memory View id with the stored decryption shares.
/// * `cipher_text_id` - Memory View id with the ciphertext.
#[no_mangle]
pub fn decrypt(
    public_key_set_id: u32,
    shares_id: u32,
    cipher_text_id: u32
) -> u32 {
    // 1. Collect decryption shares
    let mut dshares = BTreeMap::new();

    let mut shares_mv = ReadSeekCloser::open(shares_id).unwrap();
    let mut shares_encoded: Vec<u8> = Vec::new();
    let _ = shares_mv.read_to_end(&mut shares_encoded);

    // 2. Decode shares
    let shares_decoded = bytes_slice::to(shares_encoded);
    for (idx, share) in shares_decoded.iter().enumerate() {
        let bincode_dshare_bytes = reverse_bytes(share.to_vec());
        let dshare: DecryptionShare = bincode::deserialize(
            &bincode_dshare_bytes
        ).unwrap();
        dshares.insert(idx, dshare);
    }

    // 3. Load public key set
    let mut pkset_mv = ReadSeekCloser::open(public_key_set_id).unwrap();
    let mut pkset_buffer: Vec<u8> = Vec::new();
    let _ = pkset_mv.read_to_end(&mut pkset_buffer);

    let bincode_pkset_bytes = reverse_bytes(pkset_buffer);
    let public_key_set: PublicKeySet = bincode::deserialize(
        &bincode_pkset_bytes
    ).unwrap();

    // 4. Load ciphertext
    let mut ct_mv = ReadSeekCloser::open(cipher_text_id).unwrap();
    let mut cipher_text: Vec<u8> = Vec::new();
    let _ = ct_mv.read_to_end(&mut cipher_text);

    let bincode_ct_vec = reverse_bytes(cipher_text);
    let ct: Ciphertext = bincode::deserialize(&bincode_ct_vec).unwrap();

    // 5. Decrypt using combined shares
    let msg = public_key_set.decrypt(&dshares, &ct).unwrap();

    // 6. Store decrypted message and return ID
    Closer::new(&msg, true).unwrap().id
}
```

### 3.3 Utility Functions

```rust
/// Reverse bytes for endianness conversion
fn reverse_bytes(buffer: Vec<u8>) -> Vec<u8> {
    let mut buffer0 = buffer.clone();
    buffer0.reverse();
    buffer0
}
```

---

## 4. Memory View Integration

### 4.1 Memory View Operations

The blsttc module uses Taubyte's I2MV (Inter-Module Memory View) system:

```rust
use taubyte_sdk::i2mv::memview::{Closer, ReadSeekCloser};
use taubyte_sdk::utils::codec::bytes_slice;

// Open existing memory view for reading
let mut mv = ReadSeekCloser::open(id).unwrap();

// Read data
let mut buffer = Vec::new();
mv.read_to_end(&mut buffer).unwrap();

// Create new memory view for output
let output_id = Closer::new(&data, persist).unwrap().id;
```

### 4.2 Data Flow

```
Input: Memory Views
    ├── Public Key (pk_id)
    ├── Message (msg_id) / Ciphertext (cipher_text_id)
    └── Shares (shares_id)

Processing:
    ├── Read from memory views
    ├── Deserialize (bincode)
    ├── Cryptographic operation
    └── Serialize result

Output: New Memory View (return value = memory view id)
```

---

## 5. BLS Threshold Cryptography Background

### 5.1 BLS Signatures

BLS (Boneh-Lynn-Shacham) signatures are:
- **Short:** Single group element (~48 bytes for BLS12-381)
- **Deterministic:** Same message always produces same signature
- **Aggregatable:** Multiple signatures can be combined
- **Pairing-based:** Uses bilinear pairings on elliptic curves

### 5.2 Threshold Scheme

In a (t, n) threshold scheme:
- **n** parties each hold a share of the secret key
- **t** shares are required to decrypt/sign
- **t-1** or fewer shares reveal nothing about the secret

### 5.3 Key Generation

```
1. Dealer generates master secret key (SK)
2. Dealer creates polynomial f(x) of degree t-1
3. Each party i receives share sk_i = f(i)
4. Public keys: pk_i = sk_i * G (G is generator)
5. Public key set: {pk_1, pk_2, ..., pk_n}
```

### 5.4 Threshold Encryption Flow

```
1. Encrypt with public key: ct = encrypt(pk, message)
2. Each party computes decryption share: share_i = partial_decrypt(sk_i, ct)
3. Combine t shares: message = combine(shares_1...shares_t, ct)
```

---

## 6. Integration with Taubyte Components

### 6.1 VM Integration

The blsttc module is compiled to WebAssembly and runs in TVM:

```
blsttc Rust Source
    ├── taubyte-sdk imports
    ├── blsttc crate (BLS operations)
    └── Compile to WASM
        └── TVM (wazero runtime)
            └── Callable from other modules
```

### 6.2 Usage from Other Modules

```rust
// From another WASM module
use taubyte_sdk::i2mv::memview::Closer;

// Call blsttc functions (via host imports or direct call)
let encrypted_id = encrypt(pk_view_id, message_view_id);
let decrypted_id = decrypt(pkset_view_id, shares_view_id, encrypted_id);
```

### 6.3 SDK Integration

The Rust SDK provides memory view primitives used by blsttc:

```rust
// taubyte-sdk/src/i2mv/memview/mod.rs
pub struct ReadSeekCloser { pub id: u32 }
pub struct Closer { pub id: u32 }

impl ReadSeekCloser {
    pub fn open(id: u32) -> Result<Self, Error> { ... }
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> { ... }
    pub fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize, Error> { ... }
}

impl Closer {
    pub fn new(data: &[u8], persist: bool) -> Result<Self, Error> { ... }
}
```

---

## 7. Security Considerations

### 7.1 Key Management

- **Never expose private keys:** Only public keys and shares are used
- **Secure RNG:** Uses `rand::thread_rng()` for encryption
- **Share verification:** Shares should be verified before combination

### 7.2 Memory Safety

- **Memory views:** Data passed via memory views, not direct pointers
- **Serialization:** Bincode for safe serialization
- **Endianness:** Explicit byte order handling

### 7.3 Threshold Parameters

Recommended threshold settings:
- **Small network (3-10 nodes):** (2, 3) or (3, 5)
- **Medium network (10-50 nodes):** (t, 2t-1) where t ≈ n/3
- **Large network (50+ nodes):** (t, 3t) for Byzantine fault tolerance

---

## 8. Build and Deployment

### 8.1 Build Script

```bash
#!/bin/bash
# build.sh

# Build WASM module
cargo build --target wasm32-unknown-unknown --release

# Copy artifact
cp target/wasm32-unknown-unknown/release/blsttc.wasm artifact.wasm
```

### 8.2 Build Requirements

```
- Rust toolchain (stable)
- wasm32-unknown-unknown target
- taubyte-sdk dependency
```

### 8.3 Deployment

The compiled `artifact.wasm` can be:
1. Loaded into TVM for execution
2. Called from other WASM modules
3. Used via SDK wrappers

---

## 9. Use Cases

### 9.1 Secure Multi-Party Computation

```rust
// Multiple parties compute decryption shares
// No single party can decrypt alone
let shares = parties.iter().map(|p| p.partial_decrypt(&ct)).collect();
let message = combine_shares(shares, &ct);
```

### 9.2 Distributed Key Management

```rust
// Keys are distributed across nodes
// Compromise of < t nodes doesn't reveal key
let key_shares = generate_shares(n, t);
```

### 9.3 Threshold Access Control

```rust
// Require multiple approvals for sensitive operations
let approved = verify_signatures(signatures, threshold);
if approved { execute_sensitive_operation(); }
```

---

## 10. Related Components

| Component | Path | Description |
|-----------|------|-------------|
| rust-sdk | `../rust-sdk/` | SDK used for memory views |
| vm | `../vm/` | WASM runtime |
| p2p | `../p2p/` | P2P networking for distributed keys |

---

## 11. Documentation References

- **BLS Paper:** "Short Signatures from the Weil Pairing" (Boneh, Lynn, Shacham)
- **blsttc Crate:** https://crates.io/crates/blsttc
- **Taubyte SDK:** ../rust-sdk/
- **TVM:** ../vm/

---

*This document was generated as part of a comprehensive Taubyte codebase exploration.*
