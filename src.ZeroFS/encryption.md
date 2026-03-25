# Encryption: Modern File Encryption with age and XChaCha20

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.ZeroFS/age/`

---

## Table of Contents

1. [Introduction to File Encryption](#introduction-to-file-encryption)
2. [Encryption Primitives](#encryption-primitives)
3. [The age Encryption Format](#the-age-encryption-format)
4. [Key Management](#key-management)
5. [ZeroFS Encryption Implementation](#zerofs-encryption-implementation)
6. [Authenticated Encryption](#authenticated-encryption)
7. [Password-Based Encryption](#password-based-encryption)
8. [Code Examples](#code-examples)

---

## Introduction to File Encryption

### The Problem

Data at rest needs protection from:

- **Physical theft**: Stolen disks, backup tapes
- **Unauthorized access**: Compromised systems, insider threats
- **Compliance**: GDPR, HIPAA, PCI-DSS requirements
- **Privacy**: Personal data, sensitive documents

### Encryption Approaches

| Approach | Key Management | Performance | Use Case |
|----------|----------------|-------------|----------|
| **Full Disk Encryption** | OS-managed | Transparent | Laptop disks |
| **File-Level Encryption** | Per-file keys | Selective | Cloud storage |
| **Application Encryption** | Application logic | Variable | Databases |
| **Hardware Encryption** | Hardware secure element | Fast, secure | Self-encrypting drives |

### ZeroFS Approach

ZeroFS uses **file-level encryption** with:

- **XChaCha20-Poly1305**: Modern authenticated encryption
- **Argon2id**: Password-based key derivation
- **DEK/KEK hierarchy**: Key wrapping for password changes
- **Transparent operation**: Encryption at rest, decryption on read

---

## Encryption Primitives

### XChaCha20-Poly1305

**XChaCha20** is a stream cipher:

```
XChaCha20 Features:
- 256-bit key security
- 192-bit nonce (vs 96-bit for ChaCha20)
- Nonce misuse resistant (larger nonce = lower collision risk)
- Software-optimized (faster than AES without hardware support)
- Constant-time implementation (no timing side-channels)
```

**Poly1305** is a MAC (Message Authentication Code):

```
Poly1305 Features:
- 128-bit authentication tag
- Detects tampering
- One-time key per message
- Fast polynomial evaluation
```

**Combined (AEAD):**
```
Encrypt:
plaintext + key + nonce → ciphertext + tag

Decrypt:
ciphertext + tag + key + nonce → plaintext or FAIL (if tampered)
```

### Argon2id

**Argon2** is the winner of the Password Hashing Competition (2015):

```
Argon2 Variants:
- Argon2d: Data-dependent (GPU-resistant, side-channel vulnerable)
- Argon2i: Data-independent (side-channel resistant, GPU-vulnerable)
- Argon2id: Hybrid (best of both) ← ZeroFS uses this

Parameters:
- Memory (m): 64 MB (ZeroFS default)
- Iterations (t): 3 passes
- Parallelism (p): 4 threads
- Output length: 256 bits (32 bytes)
```

### Key Hierarchy

```
┌─────────────────────────────────────────┐
│       ZeroFS Key Hierarchy               │
├─────────────────────────────────────────┤
│                                         │
│  User Password                          │
│       │                                 │
│       ▼ (Argon2id KDF)                  │
│  ┌─────────────────────────────────┐    │
│  │ KEK (Key Encryption Key)        │    │
│  │ - 256 bits                      │    │
│  │ - Used only for key wrapping    │    │
│  └─────────────┬───────────────────┘    │
│                │ (XChaCha20-Poly1305)   │
│                ▼                         │
│  ┌─────────────────────────────────┐    │
│  │ DEK (Data Encryption Key)       │    │
│  │ - 256 bits                      │    │
│  │ - Used for file encryption      │    │
│  │ - Stored encrypted (wrapped)    │    │
│  └─────────────┬───────────────────┘    │
│                │ (XChaCha20-Poly1305)   │
│                ▼                         │
│  ┌─────────────────────────────────┐    │
│  │ File Data (32KB chunks)         │    │
│  │ - Compressed then encrypted     │    │
│  └─────────────────────────────────┘    │
│                                         │
└─────────────────────────────────────────┘
```

---

## The age Encryption Format

### Overview

**age** is a modern, simple file encryption tool:

```
age Design Goals:
- Small, explicit keys (age1... recipients)
- Post-quantum support (ML-KEM/X25519 hybrid)
- No config options (sensible defaults)
- UNIX-style composability
```

### age File Format

```
┌─────────────────────────────────────────┐
│          age File Structure              │
├─────────────────────────────────────────┤
│                                         │
│  Header (ASCII armor, optional)         │
│  ┌─────────────────────────────────┐    │
│  │ age-encryption.org/v1           │    │
│  │ → X25519 recipient_key          │    │
│  │ → mac: authentication_tag       │    │
│  └─────────────────────────────────┘    │
│                                         │
│  Header (binary, 64 bytes)              │
│  ┌─────────────────────────────────┐    │
│  │ Salt (32 bytes)                 │    │
│  │ Nonce (24 bytes)                │    │
│  │ Payload key (wrapped)           │    │
│  └─────────────────────────────────┘    │
│                                         │
│  Payload                                │
│  ┌─────────────────────────────────┐    │
│  │ Encrypted data blocks           │    │
│  │ - 64KB chunks                   │    │
│  │ - Each chunk has auth tag       │    │
│  └─────────────────────────────────┘    │
│                                         │
└─────────────────────────────────────────┘
```

### age Recipients

```
Recipient Types:
1. X25519 (default)
   age1ql3z7hjy54pw3hyww5ayyfg7zqgvc7w3j2elw8zmrj2kg5sfn9aqmcac8p

2. SSH RSA
   ssh-rsa AAAA...

3. SSH Ed25519
   ssh-ed25519 AAAA...

4. Post-Quantum (ML-KEM-768 + X25519)
   age1pq1... (long key)

5. Passphrase
   Scrypt-based encryption
```

### age Command Line

```bash
# Generate key
age-keygen -o key.txt
# Public key: age1ql3z7hjy54pw3hyww5ayyfg7zqgvc7w3j2elw8zmrj2kg5sfn9aqmcac8p

# Encrypt
age -r age1ql3z... -o secret.txt.age secret.txt

# Decrypt
age -d -i key.txt secret.txt.age > secret.txt

# Passphrase encryption
age -p secret.txt > secret.txt.age

# Multiple recipients
age -r age1... -r age1... secret.txt > secret.txt.age
```

---

## Key Management

### Key Generation

```rust
// From age's x25519.rs
use x25519_dalek::{PublicKey, StaticSecret};
use rand::rngs::OsRng;

pub struct Identity {
    secret_key: StaticSecret,
    public_key: PublicKey,
}

impl Identity {
    pub fn generate() -> Self {
        let mut rng = OsRng;
        let secret_key = StaticSecret::random(&mut rng);
        let public_key = PublicKey::from(&secret_key);

        Self {
            secret_key,
            public_key,
        }
    }

    pub fn to_string(&self) -> String {
        // BECH32 encoding of secret key
        format!("AGE-SECRET-KEY-1{}", bech32_encode(&self.secret_key))
    }

    pub fn recipient(&self) -> Recipient {
        Recipient {
            public_key: self.public_key,
        }
    }
}
```

### Key Wrapping

```rust
// Key wrapping with XChaCha20-Poly1305
use chacha20poly1305::{
    XChaCha20Poly1305, Key, XNonce,
    aead::{Aead, KeyInit},
};

fn wrap_key(kek: &[u8; 32], dek: &[u8; 32]) -> Result<Vec<u8>> {
    let cipher = XChaCha20Poly1305::new(Key::from_slice(kek));
    let nonce = generate_random_nonce();

    let wrapped_dek = cipher.encrypt(XNonce::from_slice(&nonce), dek.as_ref())?;

    // Return nonce + wrapped key
    let mut result = nonce.to_vec();
    result.extend(wrapped_dek);
    Ok(result)
}

fn unwrap_key(kek: &[u8; 32], wrapped: &[u8]) -> Result<[u8; 32]> {
    let nonce = XNonce::from_slice(&wrapped[..24]);
    let ciphertext = &wrapped[24..];

    let cipher = XChaCha20Poly1305::new(Key::from_slice(kek));
    let dek_vec = cipher.decrypt(nonce, ciphertext)?;

    let mut dek = [0u8; 32];
    dek.copy_from_slice(&dek_vec);
    Ok(dek)
}
```

### Password-Based Key Derivation

```rust
// From ZeroFS key_management.rs
use argon2::{
    Algorithm, Argon2, Params, Version,
    password_hash::{PasswordHasher, SaltString},
};

const ARGON2_MEM_COST: u32 = 65536;  // 64 MB
const ARGON2_TIME_COST: u32 = 3;      // 3 iterations
const ARGON2_PARALLELISM: u32 = 4;    // 4 threads

pub struct KeyManager {
    argon2: Argon2<'static>,
}

impl KeyManager {
    pub fn new() -> Self {
        let params = Params::new(
            ARGON2_MEM_COST,
            ARGON2_TIME_COST,
            ARGON2_PARALLELISM,
            None,
        ).expect("Valid Argon2 parameters");

        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
        Self { argon2 }
    }

    pub fn derive_kek(&self, password: &str, salt: &SaltString) -> Result<[u8; 32]> {
        let password_hash = self.argon2.hash_password(password.as_bytes(), salt)?;

        let hash_bytes = password_hash
            .hash
            .ok_or_else(|| anyhow::anyhow!("No hash in password hash"))?;

        let mut kek = [0u8; 32];
        kek.copy_from_slice(&hash_bytes.as_bytes()[..32]);
        Ok(kek)
    }

    pub fn generate_and_wrap_key(&self, password: &str) -> Result<(WrappedDataKey, [u8; 32])> {
        // Generate random DEK
        let mut dek = [0u8; 32];
        thread_rng().fill_bytes(&mut dek);

        // Generate random salt
        let salt = SaltString::generate(&mut thread_rng());

        // Derive KEK from password
        let kek = self.derive_kek(password, &salt)?;

        // Wrap DEK with KEK
        let wrapped_dek = wrap_key(&kek, &dek)?;

        Ok((WrappedDataKey {
            salt: salt.to_string(),
            nonce: /* from wrap_key */,
            wrapped_dek,
            version: 1,
        }, dek))
    }
}
```

### Password Change

```rust
// Change password without re-encrypting all data
pub fn rewrap_key(
    &self,
    old_password: &str,
    new_password: &str,
    wrapped_key: &WrappedDataKey,
) -> Result<WrappedDataKey> {
    // 1. Unwrap with old password
    let dek = self.unwrap_key(old_password, wrapped_key)?;

    // 2. Generate new salt and derive new KEK
    let salt = SaltString::generate(&mut thread_rng());
    let new_kek = self.derive_kek(new_password, &salt)?;

    // 3. Re-wrap DEK with new KEK
    let new_wrapped_dek = wrap_key(&new_kek, &dek)?;

    Ok(WrappedDataKey {
        salt: salt.to_string(),
        nonce: /* from new wrap */,
        wrapped_dek: new_wrapped_dek,
        version: 1,
    })
}
```

**Benefits:**
- Only re-wrap the DEK (not all file data)
- Fast password change (milliseconds)
- Same encrypted data, new key wrapper

---

## ZeroFS Encryption Implementation

### Block Transformer

```rust
// Encryption at SlateDB block level
use chacha20poly1305::{XChaCha20Poly1305, Key, XNonce, aead::{Aead, KeyInit}};

pub struct EncryptionTransformer {
    key: [u8; 32],  // DEK
}

impl BlockTransformer for EncryptionTransformer {
    fn transform(&self, block: &[u8]) -> Vec<u8> {
        let cipher = XChaCha20Poly1305::new(Key::from_slice(&self.key));

        // Generate unique nonce per block
        let nonce = generate_block_nonce(block);

        // Encrypt with authentication
        let ciphertext = cipher.encrypt(
            XNonce::from_slice(&nonce),
            block,
        ).expect("Encryption failed");

        // Prepend nonce to ciphertext
        let mut result = nonce.to_vec();
        result.extend(ciphertext);
        result
    }

    fn inverse(&self, block: &[u8]) -> Option<Vec<u8>> {
        // Extract nonce
        let nonce = XNonce::from_slice(&block[..24]);
        let ciphertext = &block[24..];

        let cipher = XChaCha20Poly1305::new(Key::from_slice(&self.key));

        // Decrypt and verify authentication tag
        cipher.decrypt(nonce, ciphertext).ok()
    }
}
```

### Key Storage

```rust
// Wrapped key stored in object store
const WRAPPED_KEY_FILENAME: &str = "zerofs.key";

pub async fn save_wrapped_key_to_object_store(
    object_store: &Arc<dyn ObjectStore>,
    db_path: &Path,
    wrapped_key: &WrappedDataKey,
) -> Result<()> {
    let key_path = wrapped_key_path(db_path);

    // Serialize wrapped key
    let serialized = bincode::serialize(wrapped_key)?;

    // Store to object store (S3, Azure, etc.)
    object_store
        .put(&key_path, PutPayload::from(Bytes::from(serialized)))
        .await?;

    Ok(())
}

pub async fn load_wrapped_key_from_object_store(
    object_store: &Arc<dyn ObjectStore>,
    db_path: &Path,
) -> Result<Option<WrappedDataKey>> {
    let key_path = wrapped_key_path(db_path);

    match object_store.get(&key_path).await {
        Ok(result) => {
            let data = result.bytes().await?;
            let wrapped_key = bincode::deserialize(&data)?;
            Ok(Some(wrapped_key))
        }
        Err(object_store::Error::NotFound { .. }) => Ok(None),
        Err(e) => Err(anyhow::anyhow!("Failed to load wrapped key: {}", e)),
    }
}
```

### Initialization Flow

```rust
pub async fn load_or_init_encryption_key(
    object_store: &Arc<dyn ObjectStore>,
    db_path: &Path,
    password: &str,
    read_only: bool,
) -> Result<[u8; 32]> {
    let key_manager = KeyManager::new();

    // Try to load existing wrapped key
    let existing_key = load_wrapped_key_from_object_store(object_store, db_path).await?;

    match existing_key {
        Some(wrapped_key) => {
            // Existing filesystem: unwrap key
            spawn_blocking_named("argon2-unwrap", move || {
                key_manager.unwrap_key(&password, &wrapped_key)
            }).await?
        }
        None => {
            // New filesystem: generate and store key
            if read_only {
                return Err(anyhow::anyhow!(
                    "Cannot initialize encryption key in read-only mode"
                ));
            }

            let (wrapped_key, dek) = spawn_blocking_named("argon2-generate", move || {
                key_manager.generate_and_wrap_key(&password)
            }).await??;

            save_wrapped_key_to_object_store(object_store, db_path, &wrapped_key).await?;
            Ok(dek)
        }
    }
}
```

### What's Encrypted

```
ZeroFS Encryption Coverage:

✓ Encrypted:
- File contents (32KB chunks)
- File metadata (permissions, timestamps, sizes)
- Extended attributes
- Symlink targets

✗ Not Encrypted:
- Key structure (inode IDs)
- Directory entry names (filenames)
- Key metadata (creation time, etc.)

Rationale:
- Encrypting keys would break LSM-tree sorting
- Filename encryption impacts directory listing performance
- Key structure reveals hierarchy but not content
- For hidden filenames, layer with gocryptfs
```

### Compression Before Encryption

```rust
// ZeroFS compresses then encrypts
use lz4_flex::compress_prepend_size;
use zstd::stream::encode_all;

enum CompressionType {
    Lz4,
    Zstd { level: i32 },
}

fn compress_and_encrypt(data: &[u8], cipher: &XChaCha20Poly1305) -> Vec<u8> {
    // 1. Compress
    let compressed = match compression_type {
        CompressionType::Lz4 => compress_prepend_size(data),
        CompressionType::Zstd(level) => encode_all(data, level).unwrap(),
    };

    // 2. Encrypt
    let nonce = generate_nonce();
    let ciphertext = cipher.encrypt(XNonce::from_slice(&nonce), &compressed).unwrap();

    // 3. Combine: nonce + ciphertext
    [nonce.as_slice(), &ciphertext].concat()
}
```

**Why compress first?**
- Compression reduces data size (saves storage, bandwidth)
- Encrypted data cannot be compressed (looks random)
- Compression before encryption is secure (no known attacks)

---

## Authenticated Encryption

### AEAD (Authenticated Encryption with Associated Data)

```
AEAD provides:
1. Confidentiality: Data is encrypted
2. Integrity: Tampering is detected
3. Authenticity: Only key holder could create

XChaCha20-Poly1305 AEAD:
┌─────────────────────────────────────────┐
│  Input:                                 │
│  - Plaintext                            │
│  - Key (256 bits)                       │
│  - Nonce (192 bits)                     │
│  - Associated Data (optional, auth only)│
├─────────────────────────────────────────┤
│  Output:                                │
│  - Ciphertext                           │
│  - Authentication Tag (128 bits)        │
└─────────────────────────────────────────┘
```

### Nonce Management

```rust
// Critical: Never reuse (key, nonce) pair!
// XChaCha20 uses 192-bit nonces for this reason

// Option 1: Counter-based nonces
struct NonceGenerator {
    counter: AtomicU64,
}

impl NonceGenerator {
    pub fn next(&self) -> [u8; 24] {
        let count = self.counter.fetch_add(1, Ordering::Relaxed);
        let mut nonce = [0u8; 24];
        nonce[..8].copy_from_slice(&count.to_le_bytes());
        nonce
    }
}

// Option 2: Random nonces (safe with 192 bits)
fn generate_random_nonce() -> [u8; 24] {
    let mut nonce = [0u8; 24];
    thread_rng().fill_bytes(&mut nonce);
    nonce
}

// Collision probability with 192-bit nonces:
// - 1 billion encryptions: ~10^-20 probability
// - Essentially zero for practical purposes
```

### Authentication Tag Verification

```rust
// Decryption automatically verifies tag
fn decrypt_and_verify(ciphertext: &[u8], key: &[u8; 32]) -> Result<Vec<u8>> {
    let nonce = XNonce::from_slice(&ciphertext[..24]);
    let encrypted = &ciphertext[24..];

    let cipher = XChaCha20Poly1305::new(Key::from_slice(key));

    match cipher.decrypt(nonce, encrypted) {
        Ok(plaintext) => Ok(plaintext),
        Err(_) => Err(anyhow::anyhow!(
            "Decryption failed: tampered data or wrong key"
        )),
    }
}

// Tag verification failures indicate:
// 1. Wrong key
// 2. Corrupted data
// 3. Malicious tampering
// In all cases: DO NOT USE THE DATA
```

---

## Post-Quantum Encryption

### The Quantum Threat

```
Current Situation:
- X25519 (ECDH): ~128-bit security
- RSA-2048: ~112-bit security
- AES-256: 256-bit security (quantum resistant)
- XChaCha20: 256-bit security (quantum resistant)

Quantum Computer Threat:
- Shor's algorithm breaks ECDH, RSA
- Grover's algorithm weakens symmetric (halves security)

Timeline:
- Estimates: 10-30 years for cryptographically relevant QC
- Harvest now, decrypt later: Data captured today could be decrypted later
```

### age Post-Quantum Support

```
age v1.3.0+ includes post-quantum hybrid encryption:

ML-KEM-768 + X25519 Hybrid:
- ML-KEM-768: NIST post-quantum KEM (Kyber)
- X25519: Classical ECDH
- Combined: Secure against both classical and quantum

Key Format:
- Recipient: age1pq1... (2000+ chars)
- Identity: AGE-SECRET-KEY-PQ-1...

Usage:
age-keygen -pq -o key.txt
age -R recipient.txt file.txt > file.txt.age
age -d -i key.txt file.txt.age > file.txt
```

### ZeroFS Post-Quantum Considerations

```rust
// Future: Post-quantum key hierarchy
pub struct PostQuantumKeyManager {
    argon2: Argon2<'static>,
    pq_kem: MlKem768,  // Future addition
}

impl PostQuantumKeyManager {
    pub fn generate_and_wrap_key_pq(&self, password: &str) -> Result<(WrappedDataKey, [u8; 32])> {
        // Generate DEK
        let mut dek = [0u8; 32];
        thread_rng().fill_bytes(&mut dek);

        // Derive KEK from password (Argon2id still PQ-safe)
        let salt = SaltString::generate(&mut thread_rng());
        let kek = self.derive_kek(password, &salt)?;

        // Wrap with hybrid PQC + classical
        let wrapped_dek = self.hybrid_wrap(&dek, &kek)?;

        Ok((wrapped_key, dek))
    }
}
```

---

## Code Examples

### Complete Encryption Example

```rust
use chacha20poly1305::{
    XChaCha20Poly1305, Key, XNonce,
    aead::{Aead, KeyInit},
};
use argon2::Argon2;
use rand::{thread_rng, RngCore};

struct FileEncryptor {
    key: [u8; 32],
}

impl FileEncryptor {
    pub fn from_password(password: &str, salt: &[u8]) -> Self {
        // Derive key from password using Argon2id
        let argon2 = Argon2::default();
        let mut key = [0u8; 32];
        argon2.hash_password_into(password.as_bytes(), salt, &mut key);
        Self { key }
    }

    pub fn encrypt(&self, plaintext: &[u8]) -> Vec<u8> {
        let cipher = XChaCha20Poly1305::new(Key::from_slice(&self.key));
        let mut nonce = [0u8; 24];
        thread_rng().fill_bytes(&mut nonce);

        let ciphertext = cipher
            .encrypt(XNonce::from_slice(&nonce), plaintext)
            .expect("Encryption failed");

        // Prepend nonce
        [&nonce, &ciphertext[..]].concat()
    }

    pub fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>> {
        if ciphertext.len() < 24 {
            return Err(anyhow::anyhow!("Ciphertext too short"));
        }

        let nonce = &ciphertext[..24];
        let encrypted = &ciphertext[24..];

        let cipher = XChaCha20Poly1305::new(Key::from_slice(&self.key));

        cipher
            .decrypt(XNonce::from_slice(nonce), encrypted)
            .map_err(|_| anyhow::anyhow!("Decryption failed: tampered or wrong key"))
    }
}

// Usage
fn main() {
    let salt = b"unique_salt_for_each_file";
    let encryptor = FileEncryptor::from_password("my_password", salt);

    let plaintext = b"Secret data!";
    let encrypted = encryptor.encrypt(plaintext);
    println!("Encrypted: {:02x?}", encrypted);

    let decrypted = encryptor.decrypt(&encrypted).unwrap();
    println!("Decrypted: {:?}", String::from_utf8(decrypted).unwrap());
}
```

### Multi-Recipient Encryption

```rust
// age-style multi-recipient encryption
use x25519_dalek::{PublicKey, StaticSecret};
use chacha20poly1305::{XChaCha20Poly1305, Key, XNonce, aead::{Aead, KeyInit}};

struct MultiRecipientEncryptor;

impl MultiRecipientEncryptor {
    pub fn encrypt(data: &[u8], recipients: &[PublicKey]) -> Vec<u8> {
        // Generate random file key
        let mut file_key = [0u8; 32];
        thread_rng().fill_bytes(&mut file_key);

        // Encrypt file key for each recipient
        let mut wrapped_keys = Vec::new();
        for recipient in recipients {
            let wrapped = self.wrap_key_for_recipient(&file_key, recipient);
            wrapped_keys.push(wrapped);
        }

        // Encrypt data with file key
        let cipher = XChaCha20Poly1305::new(Key::from_slice(&file_key));
        let nonce = generate_nonce();
        let ciphertext = cipher.encrypt(XNonce::from_slice(&nonce), data).unwrap();

        // Assemble: [wrapped_keys...] [nonce] [ciphertext]
        let mut result = Vec::new();
        for wrapped in wrapped_keys {
            result.extend(wrapped);
        }
        result.extend(nonce);
        result.extend(ciphertext);
        result
    }

    fn wrap_key_for_recipient(key: &[u8; 32], recipient: &PublicKey) -> Vec<u8> {
        // ECDH key agreement
        let ephemeral = StaticSecret::random(&mut thread_rng());
        let shared = ephemeral.diffie_hellman(recipient);

        // Derive wrapping key from shared secret
        let wrapping_key = hkdf::derive(&shared.as_bytes());

        // Wrap file key
        let cipher = XChaCha20Poly1305::new(Key::from_slice(&wrapping_key));
        let nonce = generate_nonce();
        let wrapped = cipher.encrypt(XNonce::from_slice(&nonce), key).unwrap();

        // Return: ephemeral_pubkey + nonce + wrapped_key
        [&ephemeral.to_bytes(), &nonce, &wrapped].concat()
    }
}
```

---

## Summary

### Key Takeaways

1. **XChaCha20-Poly1305** provides modern authenticated encryption:
   - 256-bit security
   - Nonce misuse resistant (192-bit nonces)
   - Fast software implementation

2. **Argon2id** is the best choice for password-based key derivation:
   - Memory-hard (GPU/ASIC resistant)
   - Side-channel resistant
   - Configurable parameters

3. **Key hierarchy** (DEK/KEK) enables:
   - Fast password changes
   - Key rotation without re-encryption
   - Separation of concerns

4. **age format** provides:
   - Simple, explicit keys
   - Multiple recipient types
   - Post-quantum support

5. **ZeroFS encryption**:
   - Always-on (no opt-out)
   - Block-level (SlateDB transformer)
   - Compress then encrypt
   - What's encrypted vs not

### Further Reading

- [age Documentation](https://age-encryption.org/)
- [RFC 8439: ChaCha20-Poly1305](https://datatracker.ietf.org/doc/html/rfc8439)
- [Argon2 RFC](https://datatracker.ietf.org/doc/html/rfc9106)
- [NIST Post-Quantum Cryptography](https://csrc.nist.gov/projects/post-quantum-cryptography)
