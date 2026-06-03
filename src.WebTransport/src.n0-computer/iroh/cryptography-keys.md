# Cryptography and Keys in Iroh

## Overview

This document provides a deep dive into the cryptographic foundations of iroh, focusing on how Ed25519 keys are used for identity, authentication, and secure communication.

**Important Note:** Iroh uses **Ed25519** (EdDSA over Curve25519), not RSA. This is a deliberate design choice for performance, security, and compactness.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.n0-computer/iroh/`

---

## Table of Contents

1. [Why Ed25519 Over RSA](#why-ed25519-over-rsa)
2. [Key Structure and Storage](#key-structure-and-storage)
3. [Key Derivation: Public from Private](#key-derivation-public-from-private)
4. [Signing and Verification](#signing-and-verification)
5. [TLS with Raw Public Keys](#tls-with-raw-public-keys)
6. [Relay Authentication](#relay-authentication)
7. [Session Resumption](#session-resumption)
8. [Security Considerations](#security-considerations)

---

## Why Ed25519 Over RSA

### Comparison Table

| Property | Ed25519 | RSA-2048 | RSA-4096 |
|----------|---------|----------|----------|
| **Private Key Size** | 32 bytes | 256+ bytes | 512+ bytes |
| **Public Key Size** | 32 bytes | 256 bytes | 512 bytes |
| **Signature Size** | 64 bytes | 256 bytes | 512 bytes |
| **Key Generation** | ~100μs | ~100ms | ~1s |
| **Signing** | ~50,000/s | ~1,000/s | ~200/s |
| **Verification** | ~15,000/s | ~5,000/s | ~1,000/s |
| **Security Level** | 128-bit | 112-bit | 140-bit |
| **Side-Channel Resistance** | Excellent | Requires care | Requires care |
| **Deterministic Signatures** | Yes | No (requires RNG) | No (requires RNG) |

### Design Implications

1. **Compact Wire Protocol**: Smaller keys and signatures mean less bandwidth
2. **Fast Connection Setup**: Sub-millisecond handshake on modern hardware
3. **Deterministic Signatures**: No RNG needed during signing (reduces attack surface)
4. **Self-Certifying Identifiers**: The public key IS the node ID

---

## Key Structure and Storage

### Internal Representation

```rust
// From iroh-base/src/key.rs

/// A public key (Ed25519)
///
/// Stored as the compressed Edwards-y coordinate
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct PublicKey(CompressedEdwardsY);

/// A secret key (Ed25519)
#[derive(Clone, zeroize::ZeroizeOnDrop)]
pub struct SecretKey(SigningKey);
```

### Memory Layout

```
SecretKey (32 bytes):
┌────────────────────────────────────────┐
│     Seed (32 bytes / 256 bits)         │
│  Used to derive scalar and prefix      │
└────────────────────────────────────────┘

PublicKey (32 bytes):
┌────────────────────────────────────────┐
│  Compressed Edwards-y (32 bytes)       │
│  Point on Curve25519                   │
└────────────────────────────────────────┘

Signature (64 bytes):
┌────────────────────────────────────────┐
│  R (32 bytes)  │  s (32 bytes)         │
│  Commitment    │  Response value       │
└────────────────────────────────────────┘
```

### Secure Key Storage

```rust
use zeroize::Zeroize;

// SecretKey uses ZeroizeOnDrop
impl Drop for SecretKey {
    fn drop(&mut self) {
        // Securely zero memory before deallocation
        self.0.zeroize();
    }
}

// Best practices for key storage
fn store_key_securely(key: &SecretKey, path: &Path) -> io::Result<()> {
    // 1. Create file with restrictive permissions (0600)
    // 2. Write key bytes
    // 3. Sync to disk
    // 4. Never log or display the key

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)  // Owner read/write only
        .open(path)?;

    file.write_all(&key.to_bytes())?;
    file.sync_all()?;

    Ok(())
}
```

### Serialization Formats

```rust
use data_encoding::{HEXLOWER, BASE32_NOPAD};
use serde::{Serialize, Deserialize};

// Hex encoding (human-readable)
pub fn public_key_to_hex(pk: &PublicKey) -> String {
    HEXLOWER.encode(pk.as_bytes())
    // Example: "ae58ff8833241ac82d6ff7611046ed67..."
}

pub fn public_key_from_hex(s: &str) -> Result<PublicKey, KeyParsingError> {
    let mut bytes = [0u8; 32];
    HEXLOWER.decode_mut(s.as_bytes(), &mut bytes)?;
    PublicKey::from_bytes(&bytes)
}

// Base32 encoding (URL-safe, case-insensitive)
pub fn public_key_to_base32(pk: &PublicKey) -> String {
    BASE32_NOPAD.encode(pk.as_bytes())
    // Example: "VIXOGF2K7N6ZAY3Q..."
}

// Binary serialization (postcard/cbor)
#[derive(Serialize, Deserialize)]
struct WireFormat {
    sender: PublicKey,
    signature: Signature,
}
```

---

## Key Derivation: Public from Private

### Ed25519 Key Generation

```rust
use ed25519_dalek::{SigningKey, VerifyingKey};
use curve25519_dalek::scalar::Scalar;
use sha2::{Sha512, Digest};

// Step 1: Generate random seed
fn generate_seed() -> [u8; 32] {
    let mut seed = [0u8; 32];
    rand::rng().fill(&mut seed);
    seed
}

// Step 2: Hash seed with SHA-512
fn hash_seed(seed: &[u8; 32]) -> [u8; 64] {
    let mut hasher = Sha512::new();
    hasher.update(seed);
    hasher.finalize().into()
}

// Step 3: Derive scalar (private key component)
fn derive_scalar(hashed: &[u8; 64]) -> Scalar {
    // "Clamping" the scalar for security
    let mut scalar_bytes = hashed[0..32].try_into().unwrap();

    // Clear the three least significant bits of the first byte
    scalar_bytes[0] &= 248;
    // Clear the most significant bit of the last byte
    scalar_bytes[31] &= 127;
    // Set the second most significant bit of the last byte
    scalar_bytes[31] |= 64;

    Scalar::from_bits(scalar_bytes)
}

// Step 4: Compute public key (base point * scalar)
fn compute_public_key(scalar: &Scalar) -> VerifyingKey {
    use curve25519_dalek::constants::ED25519_BASEPOINT_POINT;

    // Public key = base_point * private_scalar
    let public_point = ED25519_BASEPOINT_POINT * scalar;

    // Compress to 32 bytes (y-coordinate with sign bit)
    VerifyingKey::from(&public_point)
}

// Full derivation
fn keypair_from_seed(seed: [u8; 32]) -> (SigningKey, VerifyingKey) {
    let hashed = hash_seed(&seed);
    let scalar = derive_scalar(&hashed);
    let public = compute_public_key(&scalar);

    let secret = SigningKey::from_bytes(&seed);
    (secret, public)
}
```

### In Iroh

```rust
// From iroh-base/src/key.rs
impl SecretKey {
    /// Generate a new SecretKey with a randomness generator
    pub fn generate<R: CryptoRng + ?Sized>(csprng: &mut R) -> Self {
        let secret = SigningKey::generate(csprng);
        Self(secret)
    }

    /// The public key of this SecretKey
    pub fn public(&self) -> PublicKey {
        let key = self.0.verifying_key().to_bytes();
        PublicKey(CompressedEdwardsY(key))
    }
}
```

---

## Signing and Verification

### EdDSA Signing Process

```rust
use ed25519_dalek::{SigningKey, Signature};
use sha2::{Sha512, Digest};

fn sign_message(secret: &SigningKey, message: &[u8]) -> Signature {
    // Ed25519 signing:
    // 1. r = SHA512(prefix || message) mod L
    // 2. R = base_point * r
    // 3. h = SHA512(R || public_key || message) mod L
    // 4. s = (r + h * scalar) mod L
    // 5. signature = (R, s)

    secret.sign(message)
}
```

### Domain Separation

Critical security practice: Use domain separation to prevent cross-protocol attacks.

```rust
use blake3::derive_key;

/// Sign with domain separation
fn domain_sep_sign(
    secret: &SecretKey,
    domain: &str,
    message: &[u8],
) -> Signature {
    // Derive a unique key for this domain
    let domain_key = derive_key(domain, message);

    // Sign the derived key, not the original message
    secret.sign(&domain_key)
}

// Example domains used in iroh
const DOMAIN_RELAY_CHALLENGE: &str = "iroh-relay handshake v1 challenge signature";
const DOMAIN_TLS_KEY_EXPORT: &[u8] = b"iroh-relay handshake v1";

// Usage in relay authentication
fn sign_relay_challenge(secret: &SecretKey, challenge: &[u8; 16]) -> Signature {
    use blake3::derive_key;
    let msg = derive_key(DOMAIN_RELAY_CHALLENGE, challenge);
    secret.sign(&msg)
}
```

### Verification

```rust
// From iroh-base/src/key.rs
impl PublicKey {
    /// Verify a signature on a message with this public key
    pub fn verify(
        &self,
        message: &[u8],
        signature: &Signature,
    ) -> Result<(), SignatureError> {
        self.as_verifying_key()
            .verify_strict(message, &signature.0)
            .map_err(|_| SignatureError::new())
    }
}

// Strict verification prevents signature malleability
fn verify_strict_example() {
    let message = b"Hello, iroh!";
    let signature = secret_key.sign(message);

    // This will only succeed with the exact signature
    match public_key.verify(message, &signature) {
        Ok(()) => println!("Valid signature!"),
        Err(_) => println!("Invalid signature!"),
    }
}
```

---

## TLS with Raw Public Keys (RFC 7250)

### Why Raw Public Keys?

Traditional TLS uses X.509 certificates:
```
Client ──TLS──> Server
         │
         ▼
    Certificate Authority (CA)
         │
         ▼
    X.509 Certificate Chain
```

With Raw Public Keys (RFC 7250):
```
Client ──TLS──> Server
         │
         └─> Server's Ed25519 Public Key
             (No certificates, no CA)
```

### Iroh's TLS Configuration

```rust
// From iroh/src/tls.rs
pub(crate) struct TlsConfig {
    pub(crate) secret_key: SecretKey,
    cert_resolver: Arc<ResolveRawPublicKeyCert>,
    server_verifier: Arc<verifier::ServerCertificateVerifier>,
    client_verifier: Arc<verifier::ClientCertificateVerifier>,
    session_store: Arc<dyn rustls::client::ClientSessionStore>,
    crypto_provider: Arc<rustls::crypto::CryptoProvider>,
}

impl TlsConfig {
    pub(crate) fn new(
        secret_key: SecretKey,
        max_tls_tickets: usize,
        crypto_provider: Arc<rustls::crypto::CryptoProvider>,
    ) -> Self {
        Self {
            cert_resolver: Arc::new(ResolveRawPublicKeyCert::new(&secret_key)),
            server_verifier: Arc::new(verifier::ServerCertificateVerifier),
            client_verifier: Arc::new(verifier::ClientCertificateVerifier),
            session_store: Arc::new(
                rustls::client::ClientSessionMemoryCache::new(max_tls_tickets)
            ),
            crypto_provider,
            secret_key,
        }
    }
}
```

### Custom Certificate Verifier

```rust
// From iroh/src/tls/verifier.rs
struct ServerCertificateVerifier;

impl rustls::client::danger::ServerCertVerifier for ServerCertificateVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName,
        _ocsp: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, Error> {
        // In iroh, we verify the certificate contains the expected public key
        // The actual verification happens at a different layer

        // For now, we accept all certificates
        // (The peer's identity is verified through the connection)
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        // This is where we'd verify the signature matches the public key
        // In iroh, the public key IS the endpoint ID
        Ok(HandshakeSignatureValid::assertion())
    }
}
```

### 0-RTT Session Resumption

```rust
// Enable 0-RTT for faster reconnections
pub(crate) fn make_client_config(
    &self,
    keylog: bool,
) -> Result<QuicClientConfig, TlsConfigError> {
    let mut crypto = rustls::ClientConfig::builder_with_provider(
        self.crypto_provider.clone()
    )
    .with_protocol_versions(verifier::PROTOCOL_VERSIONS)?
    .dangerous()
    .with_custom_certificate_verifier(self.server_verifier.clone())
    .with_client_cert_resolver(self.cert_resolver.clone());

    // Enable session resumption
    crypto.resumption = rustls::client::Resumption::store(
        self.session_store.clone()
    );

    // Enable 0-RTT early data
    crypto.enable_early_data = true;

    if keylog {
        crypto.key_log = Arc::new(rustls::KeyLogFile::new());
    }

    let quic = QuicClientConfig::try_from(crypto)?;
    Ok(quic)
}
```

---

## Relay Authentication

### Handshake Flow

```
Client                              Relay Server
  │                                    │
  │────── WebSocket Connect ──────────>│
  │                                    │
  │<───── ServerChallenge (16 bytes) ──│
  │         (Random nonce)             │
  │                                    │
  │─ ClientAuth (pubkey, signature) ──>│
  │                                    │
  │        [Server verifies:]          │
  │        1. Public key valid?        │
  │        2. Signature valid?         │
  │        3. Not blacklisted?         │
  │                                    │
  │<───── ServerConfirmsAuth ──────────│
  │         (or ServerDeniesAuth)      │
  │                                    │
  │====== Authenticated Session =======│
```

### Client-Side Implementation

```rust
// From iroh-relay/src/protos/handshake.rs

/// Client authentication response
pub(crate) struct ClientAuth {
    pub(crate) public_key: PublicKey,
    pub(crate) signature: [u8; 64],
}

impl ClientAuth {
    /// Generate signature for the server's challenge
    pub(crate) fn new(secret_key: &SecretKey, challenge: &ServerChallenge) -> Self {
        Self {
            public_key: secret_key.public(),
            signature: secret_key.sign(&challenge.message_to_sign()).to_bytes(),
        }
    }
}

/// Server challenge
pub(crate) struct ServerChallenge {
    pub(crate) challenge: [u8; 16],
}

impl ServerChallenge {
    /// Generate a new random challenge
    pub(crate) fn new<R: CryptoRng + ?Sized>(rng: &mut R) -> Self {
        let mut challenge = [0u8; 16];
        rng.fill_bytes(&mut challenge);
        Self { challenge }
    }

    /// Message to sign (with domain separation)
    fn message_to_sign(&self) -> [u8; 32] {
        blake3::derive_key(
            "iroh-relay handshake v1 challenge signature",
            &self.challenge
        )
    }
}
```

### Server-Side Verification

```rust
impl ClientAuth {
    /// Verify the client's authentication
    pub(crate) fn verify(
        &self,
        challenge: &ServerChallenge,
    ) -> Result<(), Box<VerificationError>> {
        let message = challenge.message_to_sign();

        self.public_key
            .verify(&message, &Signature::from_bytes(&self.signature))
            .map_err(|err| {
                e!(VerificationError::SignatureInvalid {
                    source: err,
                    message: message.to_vec(),
                    signature: self.signature,
                    public_key: self.public_key
                })
            })
            .map_err(Box::new)
    }
}
```

### TLS Key Material Export (Fast Path)

When TLS key material export is available, skip the challenge-response round trip:

```rust
// From iroh-relay/src/protos/handshake.rs

/// Fast authentication using TLS key material
pub(crate) struct KeyMaterialClientAuth {
    pub(crate) public_key: PublicKey,
    pub(crate) signature: [u8; 64],
    pub(crate) key_material_suffix: [u8; 16],
}

impl KeyMaterialClientAuth {
    /// Generate auth from TLS key material
    pub(crate) fn new(
        secret_key: &SecretKey,
        io: &impl ExportKeyingMaterial,
    ) -> Option<Self> {
        let public_key = secret_key.public();

        // Export 32 bytes of key material from TLS
        // RFC 5705: Export keying material
        let key_material = io.export_keying_material(
            [0u8; 32],
            b"iroh-relay handshake v1",  // Label
            Some(secret_key.public().as_bytes()),  // Context
        )?;

        // Split: sign first 16 bytes, send last 16 bytes for verification
        let (message, suffix) = key_material.split_at(16);

        Some(Self {
            public_key,
            signature: secret_key.sign(message).to_bytes(),
            key_material_suffix: suffix.try_into().unwrap(),
        })
    }
}
```

---

## Session Resumption

### TLS Session Tickets

```rust
// Session ticket storage
use rustls::client::ClientSessionMemoryCache;

// Configure cache size
const DEFAULT_MAX_TLS_TICKETS: usize = 8 * 32;  // 8 tickets × 32 endpoints

let session_store = ClientSessionMemoryCache::new(DEFAULT_MAX_TLS_TICKETS);
```

### Session Ticket Structure

```
Session Ticket (~200 bytes):
┌─────────────────────────────────────────────────┐
│ Session ID (32 bytes)                           │
│ Cipher State (variable)                         │
│ Master Secret (48 bytes)                        │
│ Peer's PublicKey (32 bytes)                     │
│ Timestamp + Expiry                              │
└─────────────────────────────────────────────────┘
```

### 0-RTT Data

With session resumption, clients can send data immediately:

```rust
// Client with cached session
let conn = endpoint.connect(remote_id, ALPN).await?;

// Can send 0-RTT data if session is valid
let mut send_stream = conn.open_uni().await?;
send_stream.write_all(b"Hello!").await?;  // 0-RTT data

// Server receives and processes
let (mut send, mut recv) = conn.accept_bi().await?;
```

---

## Security Considerations

### Key Rotation

```rust
/// Rotate keys periodically for forward secrecy
struct KeyRotator {
    current_key: SecretKey,
    previous_key: Option<SecretKey>,
    rotation_interval: Duration,
    last_rotation: Instant,
}

impl KeyRotator {
    fn new() -> Self {
        Self {
            current_key: SecretKey::generate(&mut rand::rng()),
            previous_key: None,
            rotation_interval: Duration::from_secs(24 * 60 * 60),  // 24 hours
            last_rotation: Instant::now(),
        }
    }

    fn maybe_rotate(&mut self) {
        if Instant::now() - self.last_rotation > self.rotation_interval {
            self.previous_key = Some(std::mem::replace(
                &mut self.current_key,
                SecretKey::generate(&mut rand::rng()),
            ));
            self.last_rotation = Instant::now();
        }
    }
}
```

### Key Fingerprinting

```rust
/// Short fingerprint for display
impl PublicKey {
    pub fn fmt_short(&self) -> impl Display + Copy + 'static {
        PublicKeyShort(self.0.as_bytes()[0..5].try_into().unwrap())
    }
}

struct PublicKeyShort([u8; 5]);

impl Display for PublicKeyShort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        data_encoding::HEXLOWER.encode_write(&self.0, f)
    }
}

// Example: "ae58ff" (first 5 bytes = 10 hex chars)
```

### Signature Verification Best Practices

```rust
// ✅ DO: Use strict verification
public_key.verify_strict(message, &signature)?;

// ❌ DON'T: Use non-strict verification (allows malleability)
public_key.verify(message, &signature)?;

// ✅ DO: Use domain separation
const MY_DOMAIN: &str = "my-protocol-v1-signature";
let msg = blake3::derive_key(MY_DOMAIN, message);
secret_key.sign(&msg);

// ❌ DON'T: Sign raw user input without domain separation
secret_key.sign(user_input);

// ✅ DO: Constant-time comparison for sensitive operations
use subtle::ConstantTimeEq;
if a.ct_eq(&b).into() { /* equal */ }

// ❌ DON'T: Use == for sensitive comparisons
if a == b { /* vulnerable to timing attacks */ }
```

### Side-Channel Protection

```rust
// Iroh uses ed25519-dalek which has built-in side-channel protection

// Additional measures:
use zeroize::Zeroize;

// Zero sensitive data after use
let mut secret_bytes = secret_key.to_bytes();
// ... use secret_bytes ...
secret_bytes.zeroize();  // Secure erase
```

---

## Key Reference

### Key Sizes and Formats

```rust
// Ed25519 constants
pub const SECRET_KEY_LENGTH: usize = 32;   // 256 bits
pub const PUBLIC_KEY_LENGTH: usize = 32;   // 256 bits
pub const SIGNATURE_LENGTH: usize = 64;    // 512 bits

// Encoded formats
// Hex: 64 chars (public key), 128 chars (signature)
// Base32: 52 chars (public key), 104 chars (signature)
// Base58: ~44 chars (public key), ~88 chars (signature)
```

### Conversion Table

```
Bytes → Hex:        32 bytes → 64 hex characters
Bytes → Base32:     32 bytes → 52 base32 characters
Bytes → Base58:     32 bytes → ~44 base58 characters
Bytes → Base64:     32 bytes → 44 base64 characters
Bytes → Base64URL:  32 bytes → 44 base64url characters (no padding: 43)
```

---

## See Also

- [exploration.md](./exploration.md) - Main iroh exploration
- [p2p-ground-up.md](./p2p-ground-up.md) - Building P2P from scratch
- [iroh-ids.md](./iroh-ids.md) - Node identification
