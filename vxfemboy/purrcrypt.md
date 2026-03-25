# PurrCrypt - Cat/Dog-Themed Encryption

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/vxfemboy/purrcrypt/`

---

## Overview

**PurrCrypt** is a PGP-like file encryption tool with a unique twist: it encodes encrypted data as adorable cat or dog sounds. Behind the playful interface lies serious cryptography using the same elliptic curve algorithms (secp256k1) that secure Bitcoin.

### What It Does

1. **Generates keypairs** (public/private) using elliptic curve cryptography
2. **Encrypts files** using ECDH key exchange + AES-256-GCM
3. **Encodes ciphertext** as cat/dog sound patterns (steganography)
4. **Manages keys** in a secure directory structure with proper permissions
5. **Decrypts files** back from pet-speak to original content

### Key Features

- **Real cryptography** - secp256k1 ECDH + AES-256-GCM
- **Steganographic encoding** - Hides encrypted data as pet sounds
- **Two dialects** - Cat mode (mew, purr, nya) or Dog mode (woof, bark, arf)
- **Secure key storage** - 0o600 permissions on private keys
- **Configurable** - TOML config for preferred dialect

---

## Cryptography Pipeline

### Encryption Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                    PurrCrypt Encryption                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────┐                                               │
│  │  Plain Text  │  (Input file)                                 │
│  │   "Hello"    │                                               │
│  └──────┬───────┘                                               │
│         │                                                        │
│         ▼                                                        │
│  ┌──────────────┐                                               │
│  │   ZLIB       │  (Compression - reduces size)                 │
│  │  Compress    │                                               │
│  └──────┬───────┘                                               │
│         │                                                        │
│         ▼                                                        │
│  ┌──────────────┐                                               │
│  │   ECDH       │  (Key exchange with recipient's public key)   │
│  │  Key Derive  │  - Generate ephemeral keypair                 │
│  │              │  - Compute DH shared secret                   │
│  │              │  - HKDF extract/expand for key material       │
│  └──────┬───────┘                                               │
│         │                                                        │
│         ▼                                                        │
│  ┌──────────────┐                                               │
│  │   AES-256    │  (Authenticated encryption)                   │
│  │    -GCM      │  - Encrypt with derived key                   │
│  │              │  - Append auth tag                            │
│  └──────┬───────┘                                               │
│         │                                                        │
│         ▼                                                        │
│  ┌──────────────┐                                               │
│  │   Base64     │  (Binary to text)                             │
│  │   Encode     │                                               │
│  └──────┬───────┘                                               │
│         │                                                        │
│         ▼                                                        │
│  ┌──────────────┐                                               │
│  │  Pet Speak   │  (Steganographic encoding)                    │
│  │   Encoder    │  - 6 bits per word                            │
│  │              │  - "mew purr nya meow"                        │
│  └──────────────┘                                               │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Decryption Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                    PurrCrypt Decryption                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────┐                                               │
│  │  Pet Speak   │  "mew purr nya meow..."                       │
│  └──────┬───────┘                                               │
│         │                                                        │
│         ▼                                                        │
│  ┌──────────────┐                                               │
│  │   Pet Speak  │  (Decode words to bits)                       │
│  │   Decoder    │                                               │
│  └──────┬───────┘                                               │
│         │                                                        │
│         ▼                                                        │
│  ┌──────────────┐                                               │
│  │   Base64     │  (Text to binary)                             │
│  │   Decode     │                                               │
│  └──────┬───────┘                                               │
│         │                                                        │
│         ▼                                                        │
│  ┌──────────────┐                                               │
│  │   AES-256    │  (Authenticated decryption)                   │
│  │    -GCM      │  - Derive key from DH secret                  │
│  │              │  - Decrypt and verify tag                     │
│  └──────┬───────┘                                               │
│         │                                                        │
│         ▼                                                        │
│  ┌──────────────┐                                               │
│  │   ZLIB       │  (Decompression)                              │
│  │  Decompress  │                                               │
│  └──────┬───────┘                                               │
│         │                                                        │
│         ▼                                                        │
│  ┌──────────────┐                                               │
│  │  Plain Text  │  (Output file)                                │
│  │   "Hello"    │                                               │
│  └──────────────┘                                               │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Module Structure

```
src/
├── main.rs             # CLI parsing, command dispatch
├── lib.rs              # Module exports
├── crypto.rs           # Core encryption/decryption logic
├── keys.rs             # Keypair generation and storage
├── keystore.rs         # Key directory management
├── config.rs           # TOML configuration handling
├── debug.rs            # Debug macro for verbose mode
└── cipher/
    ├── mod.rs          # Pet-speak cipher module
    └── patterns.rs     # Pattern definitions (mew, purr, woof, bark)
```

---

## Implementation Details

### 1. Key Generation (ECDH with secp256k1)

```rust
// src/keys.rs
use k256::{
    ecdh::{diffie_hellman, EphemeralSecret},
    sha2, PublicKey, SecretKey
};
use k256::elliptic_curve::rand_core::OsRng;

pub struct KeyPair {
    pub secret_key: SecretKey,
    pub public_key: PublicKey,
}

impl KeyPair {
    pub fn new() -> Self {
        // Generate random private key using OS CSPRNG
        let secret_key = SecretKey::random(&mut OsRng);
        let scalar = secret_key.to_nonzero_scalar();
        // Derive public key from private key
        let public_key = PublicKey::from_secret_scalar(&scalar);

        Self {
            secret_key,
            public_key,
        }
    }

    pub fn save_keys(&self, pub_path: &Path, secret_path: &Path)
        -> Result<(), KeyError>
    {
        // Save public key in compressed SEC1 format (33 bytes)
        let pub_bytes = self.public_key.to_sec1_bytes();
        let encoded_pub = BASE64.encode(&pub_bytes);
        fs::write(pub_path, encoded_pub)?;

        // Public key: 0o644 (readable)
        #[cfg(unix)]
        fs::set_permissions(pub_path, fs::Permissions::from_mode(0o644))?;

        // Save private key
        let secret_bytes = self.secret_key.to_bytes();
        let encoded_secret = BASE64.encode(&secret_bytes);
        fs::write(secret_path, encoded_secret)?;

        // Private key: 0o600 (owner read/write only)
        #[cfg(unix)]
        fs::set_permissions(secret_path, fs::Permissions::from_mode(0o600))?;

        Ok(())
    }
}
```

### 2. ECDH Key Exchange + AES-256-GCM Encryption

```rust
// src/crypto.rs
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use k256::ecdh::{diffie_hellman, EphemeralSecret};

pub fn encrypt_data(
    data: &[u8],
    recipient_public_key: &PublicKey
) -> Result<Vec<u8>, KeyError> {
    // Step 1: Generate ephemeral keypair for THIS message only
    let ephemeral_secret = EphemeralSecret::random(&mut OsRng);
    let ephemeral_public = PublicKey::from(&ephemeral_secret);

    // Step 2: Perform ECDH to get shared secret
    // sender_ephemeral_private ⊗ recipient_public = shared_point
    let shared_secret = ephemeral_secret.diffie_hellman(recipient_public_key);

    // Step 3: Extract key material using HKDF with SHA-256
    let shared_secret = shared_secret.extract::<sha2::Sha256>(
        Some(b"purrcrypt-salt")
    );

    // Step 4: Expand to get encryption key (32 bytes for AES-256)
    let mut encryption_key = [0u8; 32];
    shared_secret.expand(b"encryption key", &mut encryption_key)?;

    // Step 5: Expand to get nonce (12 bytes for GCM)
    let mut nonce_bytes = [0u8; 12];
    shared_secret.expand(b"nonce", &mut nonce_bytes)?;

    // Step 6: Encrypt with AES-256-GCM
    let aes_key = Key::<Aes256Gcm>::from_slice(&encryption_key);
    let cipher = Aes256Gcm::new(aes_key);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let encrypted_data = cipher.encrypt(nonce, data)?;

    // Step 7: Combine ephemeral public key + ciphertext
    // (recipient needs ephemeral pub to compute shared secret)
    let mut result = Vec::new();
    result.extend_from_slice(&ephemeral_public.to_sec1_bytes());  // 33 bytes
    result.extend_from_slice(&encrypted_data);

    Ok(result)
}
```

### 3. Decryption with ECDH

```rust
// src/crypto.rs
pub fn decrypt_data(
    encrypted_data: &[u8],
    secret_key: &SecretKey
) -> Result<Vec<u8>, KeyError> {
    // Step 1: Split ephemeral public key (33 bytes) from ciphertext
    if encrypted_data.len() <= 33 {
        return Err(KeyError::DecryptionError(
            "Encrypted data too short".to_string()
        ));
    }

    let (ephemeral_pub_bytes, encrypted) = encrypted_data.split_at(33);

    // Step 2: Reconstruct ephemeral public key
    let ephemeral_public = PublicKey::from_sec1_bytes(ephemeral_pub_bytes)?;

    // Step 3: Perform ECDH (same shared secret as sender)
    // recipient_private ⊗ ephemeral_public = shared_point
    let point = ephemeral_public.as_affine();
    let shared_secret = diffie_hellman(
        secret_key.to_nonzero_scalar(),
        point
    );

    // Step 4: Extract key material (same as encryption)
    let shared_secret = shared_secret.extract::<sha2::Sha256>(
        Some(b"purrcrypt-salt")
    );

    // Step 5: Derive encryption key
    let mut encryption_key = [0u8; 32];
    shared_secret.expand(b"encryption key", &mut encryption_key)?;

    // Step 6: Derive nonce
    let mut nonce_bytes = [0u8; 12];
    shared_secret.expand(b"nonce", &mut nonce_bytes)?;

    // Step 7: Decrypt with AES-256-GCM
    let aes_key = Key::<Aes256Gcm>::from_slice(&encryption_key);
    let cipher = Aes256Gcm::new(aes_key);
    let nonce = Nonce::from_slice(&nonce_bytes);

    cipher.decrypt(nonce, encrypted)
}
```

### 4. Pet-Speak Encoding (Steganography)

```rust
// src/cipher/mod.rs
pub struct AnimalCipher {
    cat_patterns: Vec<CipherPattern>,
    dog_patterns: Vec<CipherPattern>,
    current_dialect: CipherDialect,
}

impl AnimalCipher {
    pub fn new(dialect: CipherDialect) -> Self {
        match dialect {
            CipherDialect::Cat => Self {
                cat_patterns: vec![
                    CipherPattern::new_complex("mew", "m", 1, 4, "e", 1, 4, "w", 1, 4),
                    CipherPattern::new_complex("purr", "p", 1, 4, "u", 1, 4, "r", 1, 4),
                    CipherPattern::new_complex("nya", "n", 1, 4, "y", 1, 4, "a", 1, 4),
                    CipherPattern::new_special("meow"),
                    CipherPattern::new_complex("mrrp", "m", 1, 4, "r", 1, 4, "p", 1, 4),
                ],
                dog_patterns: vec![/* similar */],
                current_dialect: CipherDialect::Cat,
            },
            CipherDialect::Dog => { /* similar */ }
        }
    }

    /// Encode bytes as pet-speak words
    pub fn process_data<W: Write>(
        &self,
        data: &[u8],
        writer: &mut W,
        _mode: CipherMode,
    ) -> io::Result<()> {
        let mut i = 0;
        while i < data.len() {
            let remaining = data.len() - i;

            if remaining >= 3 {
                // Process 3 bytes (24 bits) -> 4 words (6 bits each)
                let byte1 = data[i];
                let byte2 = data[i + 1];
                let byte3 = data[i + 2];

                // Pack into 24-bit value
                let packed_value = ((byte1 as u32) << 16)
                    | ((byte2 as u32) << 8)
                    | (byte3 as u32);

                // Extract 4 groups of 6 bits
                let group1 = ((packed_value >> 18) & 0x3F) as u8;
                let group2 = ((packed_value >> 12) & 0x3F) as u8;
                let group3 = ((packed_value >> 6) & 0x3F) as u8;
                let group4 = (packed_value & 0x3F) as u8;

                // Encode each 6-bit group as a word
                let word1 = self.encode_word(group1, 0)?;
                writer.write_all(word1.as_bytes())?;
                writer.write_all(b" ")?;

                let word2 = self.encode_word(group2, 1)?;
                writer.write_all(word2.as_bytes())?;
                writer.write_all(b" ")?;

                let word3 = self.encode_word(group3, 2)?;
                writer.write_all(word3.as_bytes())?;
                writer.write_all(b" ")?;

                let word4 = self.encode_word(group4, 3)?;
                writer.write_all(word4.as_bytes())?;
                writer.write_all(b" ")?;

                i += 3;
            }
            // Handle 2-byte and 1-byte cases similarly...
        }
        Ok(())
    }
}
```

### 5. Pattern Encoding (How "meow" Encodes Bits)

```rust
// src/cipher/patterns.rs
impl CipherPattern {
    pub fn generate_variation(&self, bits: u8) -> String {
        match self.pattern_type {
            PatternVariation::Special => {
                if self.prefix == "m" {
                    // "meow" pattern - encode 6 bits across 4 letters
                    let m_count = ((bits >> 4) & 0x03) + 1;  // Bits 5-4: 1-4 'm's
                    let e_count = ((bits >> 2) & 0x03) + 1;  // Bits 3-2: 1-4 'e's
                    let o_count = ((bits >> 1) & 0x01) + 1;  // Bit 1: 1-2 'o's
                    let w_count = (bits & 0x01) + 1;         // Bit 0: 1-2 'w's

                    format!(
                        "{}{}{}{}",
                        "m".repeat(m_count as usize),
                        "e".repeat(e_count as usize),
                        "o".repeat(o_count as usize),
                        "w".repeat(w_count as usize)
                    )
                }
            }
            PatternVariation::Complex => {
                // "mew", "purr", "nya" patterns - 6 bits across 3 letters
                let prefix_count = ((bits >> 4) & 0x03) + 1;
                let middle_count = ((bits >> 2) & 0x03) + 1;
                let suffix_count = (bits & 0x03) + 1;

                format!("{}{}{}",
                    self.prefix.repeat(prefix_count as usize),
                    self._middle_prefix.repeat(middle_count as usize),
                    self.suffix.repeat(suffix_count as usize)
                )
            }
        }
    }

    pub fn decode_variation(&self, word: &str) -> Option<u8> {
        // Count character repetitions to recover bits
        // "mmmeeeowww" -> count m's, e's, o's, w's -> reconstruct 6 bits
        if self.prefix == "m" {
            if word.contains('m') && word.contains('e') {
                let m_count = word.chars().filter(|&c| c == 'm')
                    .count().clamp(1, 4) - 1;
                let e_count = word.chars().filter(|&c| c == 'e')
                    .count().clamp(1, 4) - 1;
                let o_count = if word.contains('o') {
                    word.chars().filter(|&c| c == 'o').count().clamp(1, 2) - 1
                } else { 0 };
                let w_count = if word.contains('w') {
                    word.chars().filter(|&c| c == 'w').count().clamp(1, 2) - 1
                } else { 0 };

                Some((m_count << 4 | e_count << 2 | o_count << 1 | w_count) as u8)
            }
        }
        // ...
    }
}
```

### 6. Keystore Management

```rust
// src/keystore.rs
pub struct Keystore {
    pub home_dir: PathBuf,   // ~/.purr
    pub keys_dir: PathBuf,   // ~/.purr/keys
}

impl Keystore {
    pub fn new() -> Result<Self, KeystoreError> {
        let home = dirs::home_dir().ok_or(KeystoreError::NoHomeDir)?;
        let purr_dir = home.join(".purr");
        let keys_dir = purr_dir.join("keys");

        // Create directory structure
        fs::create_dir_all(&keys_dir)?;
        fs::create_dir_all(keys_dir.join("public"))?;
        fs::create_dir_all(keys_dir.join("private"))?;

        // Set secure permissions (Unix only)
        #[cfg(unix)]
        {
            // ~/.purr: 0o700
            fs::set_permissions(&purr_dir, fs::Permissions::from_mode(0o700))?;
            // ~/.purr/keys: 0o700
            fs::set_permissions(&keys_dir, fs::Permissions::from_mode(0o700))?;
            // ~/.purr/keys/private: 0o700
            fs::set_permissions(&keys_dir.join("private"),
                fs::Permissions::from_mode(0o700))?;
            // ~/.purr/keys/public: 0o755
            fs::set_permissions(&keys_dir.join("public"),
                fs::Permissions::from_mode(0o755))?;
        }

        Ok(Self {
            home_dir: purr_dir,
            keys_dir,
        })
    }

    pub fn verify_permissions(&self) -> Result<(), KeystoreError> {
        for entry in fs::read_dir(self.keys_dir.join("private"))? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            let mode = metadata.permissions().mode();

            if mode & 0o077 != 0 {
                return Err(KeystoreError::InvalidPermissions(format!(
                    "Private key {} has unsafe permissions: {:o}",
                    entry.path().display(), mode
                )));
            }
        }
        Ok(())
    }
}
```

---

## Dependencies

```toml
[package]
name = "purrcrypt"
version = "0.1.0"
edition = "2021"
authors = ["sad <sad@im.gay>"]
description = "A cat-and-dog-themed pgp-like encryption tool"

[[bin]]
name = "purr"
path = "src/main.rs"

[dependencies]
flate2 = "1.0.35"           # ZLIB compression
regex = "1.11.1"            # Pattern matching
elliptic-curve = "0.13.8"   # EC traits
rand_core = "0.6.4"         # CSPRNG traits
rand = "0.9.0"              # Random number generation
k256 = { version = "0.13.3", features = ["ecdh"] }  # secp256k1
aes-gcm = "0.10.3"          # AES-256-GCM
base64 = "0.22.1"           # Base64 encoding
thiserror = "2.0.12"        # Error type derivation
dirs = "6.0.0"              # Home directory
serde = { version = "1.0.130", features = ["derive"] }
serde_json = "1.0.68"
toml = "0.8.20"             # Config file format
```

---

## Usage

### Installation

```bash
git clone https://github.com/vxfemboy/purrcrypt.git
cd purrcrypt
cargo install --path .
```

### Generate Keypair

```bash
purr genkey fluffy
# Creates:
#   ~/.purr/keys/public/fluffy.pub
#   ~/.purr/keys/private/fluffy.key
```

### Import Friend's Public Key

```bash
purr import-key --public ~/Downloads/mr_whiskers_key.pub
```

### Set Dialect

```bash
purr set-dialect cat   # or dog
```

### Encrypt File

```bash
# Using positional argument
purr encrypt -r mr_whiskers secret.txt

# Using --input flag
purr encrypt -r mr_whiskers -i secret.txt

# Override dialect for this encryption
purr encrypt -r mr_whiskers -i secret.txt --dialect dog

# Output: secret.txt.purr
```

### Decrypt File

```bash
purr decrypt -k fluffy -i secret.txt.purr
# Output: secret.txt.purr.decrypted (or custom -o)
```

### List Keys

```bash
purr list-keys
```

---

## Example Encrypted Output

### Cat Mode

```
mew purrrr nyaaa meoww purr nyaa meeww purrr nya meww meow purrrr
nyaa meow purr nya meow purrr nyaaa mew purr mrrp purrrr nyaa
```

### Dog Mode

```
woof bark arff yipp woooof baark arfff wooof barkkk arff woooof
barkk arff woof bark yippp wooof barkkk arfff yipp wooof barkk
```

---

## Command Reference

```
purr - Because "woof" and "meow" are actually secret codes!

Usage:
    purr [COMMAND] [OPTIONS]

Commands:
    genkey [name]                   Generate a new keypair
    import-key [--public] <keyfile> Import a key
    encrypt, -e                     Encrypt a message
    decrypt, -d                     Decrypt a message
    list-keys, -k                   List known keys
    set-dialect <cat|dog>          Set preferred dialect
    verbose, -v                     Enable verbose debug output

Options for encrypt:
    -r, --recipient <key>          Recipient's public key or name
    -o, --output <file>            Output file (default: adds .purr)
    -i, --input <file>             Input file
    --dialect <cat|dog>            Override dialect for this encryption

Options for decrypt:
    -k, --key <key>               Your private key or name
    -o, --output <file>           Output file
    -i, --input <file>            Input file
```

---

## Security Considerations

### What PurrCrypt Gets Right

1. **ECDH with Ephemeral Keys** - Each message uses a new ephemeral keypair
2. **HKDF Key Derivation** - Proper key material extraction with salt
3. **AES-256-GCM** - Authenticated encryption with integrity verification
4. **Secure Permissions** - Private keys stored with 0o600
5. **CSPRNG** - Uses OS random source (OsRng)

### Limitations

1. **No Key Signing** - No web of trust or key verification
2. **No Passphrase Protection** - Private keys not encrypted at rest
3. **Steganography ≠ Security** - Pet-speak encoding is obscurity, not security
4. **No Forward Secrecy** - Compromised key can decrypt all past messages

---

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let keypair = KeyPair::new();
        let data = b"Hello FBI, i'm a cat";

        let encrypted = encrypt_data(data, &keypair.public_key)
            .expect("Encryption should succeed");

        let decrypted = decrypt_data(&encrypted, &keypair.secret_key)
            .expect("Decryption should succeed");

        assert_eq!(data.to_vec(), decrypted);
    }

    #[test]
    fn test_basic_encryption_decryption() {
        let cipher = AnimalCipher::new(CipherDialect::Cat);
        let test_data = b"Hello, World!";

        let mut encrypted = Vec::new();
        cipher.process_data(test_data, &mut encrypted, CipherMode::Encrypt)
            .unwrap();
        let encrypted_str = String::from_utf8(encrypted.clone()).unwrap();

        let decrypted = cipher.process_string(&encrypted_str, CipherMode::Decrypt)
            .unwrap();

        assert_eq!(test_data.as_slice(), decrypted.as_slice());
    }
}
```

---

## Files

- **Main Entry:** `/home/darkvoid/Boxxed/@formulas/src.rust/vxfemboy/purrcrypt/src/main.rs`
- **Library:** `/home/darkvoid/Boxxed/@formulas/src.rust/vxfemboy/purrcrypt/src/lib.rs`
- **Crypto:** `/home/darkvoid/Boxxed/@formulas/src.rust/vxfemboy/purrcrypt/src/crypto.rs`
- **Keys:** `/home/darkvoid/Boxxed/@formulas/src.rust/vxfemboy/purrcrypt/src/keys.rs`
- **Keystore:** `/home/darkvoid/Boxxed/@formulas/src.rust/vxfemboy/purrcrypt/src/keystore.rs`
- **Config:** `/home/darkvoid/Boxxed/@formulas/src.rust/vxfemboy/purrcrypt/src/config.rs`
- **Cipher:** `/home/darkvoid/Boxxed/@formulas/src.rust/vxfemboy/purrcrypt/src/cipher/mod.rs`
- **Patterns:** `/home/darkvoid/Boxxed/@formulas/src.rust/vxfemboy/purrcrypt/src/cipher/patterns.rs`
- **Cargo.toml:** `/home/darkvoid/Boxxed/@formulas/src.rust/vxfemboy/purrcrypt/Cargo.toml`
- **Documentation:** `/home/darkvoid/Boxxed/@formulas/src.rust/vxfemboy/purrcrypt/README.md`

---

## Summary

PurrCrypt demonstrates:

1. **Proper ECDH key exchange** using secp256k1 (same curve as Bitcoin)
2. **HKDF key derivation** with extract/expand for key material
3. **AES-256-GCM** authenticated encryption
4. **Secure key storage** with Unix permission handling
5. **Custom encoding schemes** for steganographic output
6. **Comprehensive error handling** with thiserror
7. **CLI argument parsing** without external crates (manual parsing)

It's an excellent example of implementing real cryptography in Rust while having fun with the output format.
