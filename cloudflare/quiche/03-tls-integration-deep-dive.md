---
title: "TLS Integration Deep Dive"
subtitle: "TLS 1.3 handshakes, key derivation, and rotation in quiche"
---

# TLS Integration Deep Dive

## Introduction

This document provides a comprehensive deep dive into TLS 1.3 integration in quiche. We'll explore the handshake process, key derivation, key rotation, and how quiche integrates with BoringSSL.

## Table of Contents

1. [TLS 1.3 Overview](#1-tls-13-overview)
2. [Handshake Process](#2-handshake-process)
3. [Key Derivation](#3-key-derivation)
4. [Key Rotation](#4-key-rotation)
5. [0-RTT Early Data](#5-0-rtt-early-data)
6. [BoringSSL Integration](#6-boringssl-integration)
7. [Certificate Verification](#7-certificate-verification)

---

## 1. TLS 1.3 Overview

### 1.1 TLS 1.3 vs TLS 1.2

**TLS 1.2 Problems:**
- Multiple round trips (2-RTT minimum)
- Legacy cipher suites (RC4, 3DES, SHA-1)
- Complex renegotiation
- No key rotation mechanism

**TLS 1.3 Improvements:**
- 1-RTT handshake (0-RTT for returning clients)
- Simplified cipher suites (AES-GCM, ChaCha20-Poly1305)
- Perfect forward secrecy by default
- Built-in key update mechanism

### 1.2 Cipher Suites

```rust
// From quiche/src/crypto/mod.rs
pub enum Algorithm {
    AES128_GCM,
    AES256_GCM,
    ChaCha20_Poly1305,
}

impl Algorithm {
    pub const fn key_len(self) -> usize {
        match self {
            Algorithm::AES128_GCM => 16,
            Algorithm::AES256_GCM => 32,
            Algorithm::ChaCha20_Poly1305 => 32,
        }
    }

    pub const fn tag_len(self) -> usize {
        16  // All AEAD algorithms use 128-bit tags
    }

    pub const fn nonce_len(self) -> usize {
        12  // 96-bit nonces
    }
}
```

### 1.3 TLS in QUIC

TLS 1.3 in QUIC differs from TLS over TCP:

```
Traditional TLS over TCP:
- TLS records over TCP stream
- Handshake messages may span multiple TCP segments
- TLS manages its own fragmentation

TLS in QUIC:
- CRYPTO frames carry TLS data
- QUIC manages fragmentation and reliability
- TLS handshake messages map to QUIC packet number spaces
```

---

## 2. Handshake Process

### 2.1 QUIC-TLS Handshake Flow

```
Client                          Server
   |                              |
   |-------- Initial ------------>| ClientHello
   |         (ClientHello)        |
   |                              |
   |<------- Initial ------------>| ServerHello
   |         (ServerHello)        |
   |                              |
   |<------- Handshake ---------->| EncryptedExtensions
   |         (cert, CV, finished) |
   |                              |
   |-------- Handshake --------->| Handshake finished
   |         (finished)           |
   |                              |
   |======== 1-RTT packets ======| Application data
```

### 2.2 Packet Number Spaces

Each TLS epoch maps to a QUIC packet number space:

```rust
// From quiche/src/packet.rs
pub enum Epoch {
    Initial     = 0,  // ClientHello, ServerHello
    Handshake   = 1,  // Certificate, Finished
    Application = 2,  // 1-RTT application data
}

impl Epoch {
    pub fn from_packet_type(ty: Type) -> Result<Epoch> {
        match ty {
            Type::Initial => Ok(Epoch::Initial),
            Type::Handshake => Ok(Epoch::Handshake),
            Type::Short => Ok(Epoch::Application),
            _ => Err(Error::InvalidPacket),
        }
    }
}
```

### 2.3 CRYPTO Frames

TLS data is carried in CRYPTO frames:

```rust
// From quiche/src/frame.rs
pub enum Frame {
    Crypto {
        data: RangeBuf,  // TLS handshake data
    },
    CryptoHeader {
        offset: u64,  // Offset in crypto stream
        length: usize,
    },
}

// CRYPTO frame format
// 0                   1                   2                   3
// 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
//+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//| Type (0x06) | Offset (i) | Length (i) | Crypto Data (*)    ...
//+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

### 2.4 Handshake Implementation

```rust
// From quiche/src/tls/mod.rs
pub struct Context(*mut SSL_CTX);

impl Context {
    pub fn new() -> Result<Context> {
        unsafe {
            let ctx_raw = SSL_CTX_new(TLS_method());
            let mut ctx = Context(ctx_raw);
            ctx.set_session_callback();
            ctx.load_ca_certs()?;
            Ok(ctx)
        }
    }

    pub fn new_handshake(&mut self) -> Result<Handshake> {
        unsafe {
            let ssl = SSL_new(self.as_mut_ptr());
            Ok(Handshake::new(ssl))
        }
    }
}

pub struct Handshake {
    ptr: *mut SSL,
    /// Queue of data to send
    send_queue: VecDeque<Vec<u8>>,
    /// Current handshake state
    state: State,
}

impl Handshake {
    /// Process incoming TLS data
    pub fn provide_data(
        &mut self,
        level: crypto::Level,
        data: &[u8],
    ) -> Result<()> {
        // Feed data to BoringSSL
        unsafe {
            let quic_method = SSL_quic_method(self.ptr);
            quic_method.set_read_secret.unwrap()(
                self.ptr,
                level as _,
                SSL_encryption_level_e::ssl_encryption_initial,
                data.as_ptr(),
                data.len(),
            )
        }
    }

    /// Get outgoing TLS data
    pub fn process(&mut self) -> Result<()> {
        // Drive BoringSSL state machine
        unsafe {
            let ret = SSL_do_handshake(self.ptr);
            if ret <= 0 {
                let err = SSL_get_error(self.ptr, ret);
                match err {
                    SSL_ERROR_WANT_READ | SSL_ERROR_WANT_WRITE => {
                        // Need more data
                        Ok(())
                    }
                    _ => Err(Error::TlsFail),
                }
            } else {
                // Handshake complete
                self.state = State::Complete;
                Ok(())
            }
        }
    }
}
```

---

## 3. Key Derivation

### 3.1 TLS 1.3 Key Schedule

```
TLS 1.3 Key Schedule (RFC 8446 Section 7):

0 (PSK, 0-RTT)
│
├─► Early Secret
│   └─► client_early_traffic_secret
│       └─► 0-RTT keys
│
│ (0)
│
├─► Handshake Secret
│   ├─► client_handshake_traffic_secret
│   │   └─► Initial write keys (ClientHello)
│   │
│   └─► server_handshake_traffic_secret
│       └─► Handshake write keys (ServerHello+)
│
│ (0)
│
└─► Master Secret
    ├─► client_application_traffic_secret
    │   └─► 1-RTT client write keys
    │
    └─► server_application_traffic_secret
        └─► 1-RTT server write keys
```

### 3.2 Key Derivation in quiche

```rust
// From quiche/src/crypto/mod.rs
pub struct Open {
    alg: Algorithm,
    secret: Vec<u8>,
    header: HeaderProtectionKey,
    packet: PacketKey,
}

impl Open {
    pub fn from_secret(aead: Algorithm, secret: &[u8]) -> Result<Open> {
        // Derive key and IV from secret using HKDF
        let key = hkdf_expand_label(secret, b"quic key", aead.key_len())?;
        let iv = hkdf_expand_label(secret, b"quic iv", aead.nonce_len())?;
        let hp_key = hkdf_expand_label(secret, b"quic hp", aead.key_len())?;

        Ok(Open {
            alg: aead,
            secret: secret.to_vec(),
            header: HeaderProtectionKey::new(aead, hp_key)?,
            packet: PacketKey::new(aead, key, iv, Self::DECRYPT)?,
        })
    }

    /// Derive next packet key (for key update)
    pub fn derive_next_packet_key(&self) -> Result<Open> {
        // HKDF(traffic_secret, "quic ku", L)
        let next_secret = derive_next_secret(self.alg, &self.secret)?;

        // Derive new key/IV from new secret
        let next_packet_key =
            PacketKey::from_secret(self.alg, &next_secret, Self::DECRYPT)?;

        Ok(Open {
            alg: self.alg,
            secret: next_secret,
            header: self.header.clone(),  // HP key doesn't change
            packet: next_packet_key,
        })
    }
}

fn hkdf_expand_label(
    secret: &[u8],
    label: &[u8],
    length: usize,
) -> Result<Vec<u8>> {
    // HKDF-Expand per RFC 5869
    // Label = "quic " + label
    let mut hkdf_label = Vec::new();
    hkdf_label.put_u16(length as u16)?;
    hkdf_label.put_u8(label.len() as u8 + 5)?;
    hkdf_label.put_slice(b"quic ")?;
    hkdf_label.put_slice(label)?;
    hkdf_label.put_u8(0)?;  // Empty context

    hkdf_expand(secret, &hkdf_label, length)
}
```

### 3.3 Key Storage

```rust
// From quiche/src/lib.rs
struct CryptoSpace {
    /// Decryption key
    open: Option<crypto::Open>,
    /// Encryption key
    seal: Option<crypto::Seal>,
    /// Whether we've received the peer's keys
    peer_complete: bool,
    /// Whether we've sent our keys
    local_complete: bool,
}

pub struct Connection {
    /// Crypto states for each epoch
    crypto: [CryptoSpace; packet::Epoch::count()],
    /// TLS handshake state
    handshake: tls::Handshake,
}
```

---

## 4. Key Rotation

### 4.1 Key Update Mechanism

TLS 1.3 supports key updates without renegotiation:

```
Key Update Flow:
Sender                          Receiver
   |                              |
   | Update traffic secret       |
   | Derive new keys             |
   |                              |
   | Send with KEY_PHASE flip    |
   | (new packet number space)   |
   |                              |
   |                              | Detect KEY_PHASE change
   |                              | Update traffic secret
   |                              | Derive new keys
   |                              |
   |<===== Encrypted data =======>| (with new keys)
```

### 4.2 Key Phase Bit

```rust
// From quiche/src/packet.rs
const KEY_PHASE_BIT: u8 = 0x04;

// Short header packet with key phase
// 0                   1
// 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5
//+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//|1|  Reserved   |K| PN Len      |
//+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// K = Key Phase bit
// 0 = Current phase
// 1 = Next phase
```

### 4.3 Key Update Implementation

```rust
// From quiche/src/lib.rs
impl Connection {
    /// Initiate key update
    pub fn initiate_key_update(&mut self) -> Result<()> {
        // Can only update 1-RTT keys
        let epoch = packet::Epoch::Application;

        if !self.crypto[epoch].peer_complete {
            return Err(Error::KeyUpdate);
        }

        // Derive next keys
        let new_open = self.crypto[epoch]
            .open
            .as_ref()
            .ok_or(Error::KeyUpdate)?
            .derive_next_packet_key()?;

        let new_seal = self.crypto[epoch]
            .seal
            .as_ref()
            .ok_or(Error::KeyUpdate)?
            .derive_next_packet_key()?;

        // Swap keys
        self.crypto[epoch].open = Some(new_open);
        self.crypto[epoch].seal = Some(new_seal);

        // Flip key phase bit for next packet
        self.key_phase = !self.key_phase;

        Ok(())
    }

    /// Handle key phase change from peer
    fn handle_key_phase_change(
        &mut self,
        new_phase: bool,
    ) -> Result<()> {
        if new_phase != self.peer_key_phase {
            // Peer initiated key update
            let epoch = packet::Epoch::Application;

            // Derive next keys
            let new_open = self.crypto[epoch]
                .open
                .as_ref()
                .ok_or(Error::KeyUpdate)?
                .derive_next_packet_key()?;

            self.crypto[epoch].open = Some(new_open);
            self.peer_key_phase = new_phase;
        }

        Ok(())
    }
}
```

### 4.4 Key Update Limits

```rust
// From quiche/src/lib.rs
// Maximum packets before key update required
// Based on AES-GCM confidentiality limits
const MAX_AES_GCM_PACKETS: u64 = 2^48;  // ~281 trillion packets

// Proactive key update threshold
const KEY_UPDATE_PACKET_THRESHOLD: u64 = 2^40;  // Update after 1 trillion

impl Connection {
    /// Check if key update needed
    fn should_update_keys(&self) -> bool {
        let epoch = packet::Epoch::Application;
        let packets_sent = self.packets_sent[epoch];

        packets_sent >= KEY_UPDATE_PACKET_THRESHOLD
    }
}
```

---

## 5. 0-RTT Early Data

### 5.1 0-RTT Handshake

```
Client (returning)              Server
   |                              |
   |-------- Initial ------------>| ClientHello + PSK
   |         (with 0-RTT data)    |
   |                              |
   |<------- Initial ------------>| ServerHello
   |                              |
   |<------- Handshake ---------->| Finished
   |                              |
   |======== 1-RTT packets ======|
```

### 5.2 0-RTT Configuration

```rust
// From quiche/src/lib.rs
impl Config {
    /// Enable 0-RTT early data
    pub fn enable_early_data(&mut self) {
        self.tls_ctx.set_early_data_enabled(true);
    }

    /// Set session ticket key (for resumption)
    pub fn set_ticket_key(&mut self, key: &[u8]) -> Result<()> {
        self.tls_ctx.set_ticket_key(key)
    }
}

// From quiche/src/tls/mod.rs
impl Context {
    pub fn set_early_data_enabled(&mut self, enabled: bool) {
        unsafe {
            if enabled {
                SSL_CTX_set_early_data_enabled(self.0, 1);
            } else {
                SSL_CTX_set_early_data_enabled(self.0, 0);
            }
        }
    }
}
```

### 5.3 0-RTT API

```rust
impl Connection {
    /// Check if 0-RTT is possible
    pub fn is_in_early_data(&self) -> bool {
        self.handshake.is_early_data_enabled()
    }

    /// Send 0-RTT data
    pub fn stream_send_early_data(
        &mut self,
        stream_id: u64,
        buf: &[u8],
    ) -> Result<usize> {
        if !self.is_in_early_data() {
            return Err(Error::InvalidState);
        }

        // Send on stream with 0-RTT keys
        self.stream_send(stream_id, buf, false)
    }

    /// Check if 0-RTT was rejected
    pub fn early_data_rejected(&self) -> bool {
        self.handshake.early_data_rejected()
    }
}
```

### 5.4 0-RTT Security Considerations

```rust
// 0-RTT has replay attack risks
// Applications must ensure idempotency

impl Connection {
    /// Validate 0-RTT request is safe
    fn validate_early_data_request(
        &self,
        method: &[u8],
        path: &[u8],
    ) -> bool {
        // Only allow safe methods for 0-RTT
        match method {
            b"GET" | b"HEAD" | b"OPTIONS" => true,
            b"POST" => {
                // POST might be safe if idempotent
                // Check path for known idempotent endpoints
                path.starts_with(b"/api/idempotent/")
            }
            _ => false,  // PUT, DELETE, etc. not safe for 0-RTT
        }
    }
}
```

---

## 6. BoringSSL Integration

### 6.1 QUIC Method

BoringSSL has a special QUIC method:

```rust
// From quiche/src/tls/boringssl.rs
extern "C" {
    fn SSL_quic_method(ssl: *mut SSL) -> *const SSL_QUIC_METHOD;
}

static QUIC_METHOD: SSL_QUIC_METHOD = SSL_QUIC_METHOD {
    set_read_secret: Some(quic_set_read_secret),
    set_write_secret: Some(quic_set_write_secret),
    add_handshake_message: Some(quic_add_handshake_message),
    flush_flight: Some(quic_flush_flight),
    send_alert: Some(quic_send_alert),
};

unsafe extern "C" fn quic_set_read_secret(
    ssl: *mut SSL,
    level: ssl_encryption_level_t,
    cipher: *const SSL_CIPHER,
    secret: *const u8,
    secret_len: usize,
) -> ssl_private_key_result_t {
    // Called when BoringSSL has derived read keys
    let conn = SSL_get_ex_data(ssl, QUICHE_EX_DATA_INDEX) as *mut Connection;

    let alg = cipher_to_algorithm(cipher);
    let secret_slice = slice::from_raw_parts(secret, secret_len);

    // Derive QUIC keys from TLS secret
    let open = crypto::Open::from_secret(alg, secret_slice);
    (*conn).crypto_space[level].open = open.ok();

    ssl_private_key_result_t::ssl_private_key_success
}
```

### 6.2 Cipher Selection

```rust
// From quiche/src/tls/boringssl.rs
fn cipher_to_algorithm(cipher: *const SSL_CIPHER) -> crypto::Algorithm {
    unsafe {
        let name = SSL_CIPHER_get_name(cipher);
        let name_str = CStr::from_ptr(name).to_str().unwrap();

        match name_str {
            "TLS_AES_128_GCM_SHA256" => crypto::Algorithm::AES128_GCM,
            "TLS_AES_256_GCM_SHA384" => crypto::Algorithm::AES256_GCM,
            "TLS_CHACHA20_POLY1305_SHA256" => {
                crypto::Algorithm::ChaCha20_Poly1305
            }
            _ => crypto::Algorithm::AES128_GCM,  // Default
        }
    }
}
```

### 6.3 Session Management

```rust
// From quiche/src/tls/mod.rs
extern "C" fn new_session(
    ssl: *mut SSL,
    session: *mut SSL_SESSION,
) -> c_int {
    // Called when server provides session ticket
    let conn = unsafe {
        SSL_get_ex_data(ssl, QUICHE_EX_DATA_INDEX) as *mut Connection
    };

    // Store session for future 0-RTT
    unsafe {
        (*conn).session = Some(Box::new(session));
    }

    1  // Success
}

impl Connection {
    /// Get session ticket for resumption
    pub fn session(&self) -> Option<&SSL_SESSION> {
        self.session.as_ref().map(|s| s.as_ref())
    }
}
```

---

## 7. Certificate Verification

### 7.1 Certificate Loading

```rust
// From quiche/src/lib.rs
impl Config {
    pub fn load_cert_chain_from_pem_file(&mut self, file: &str) -> Result<()> {
        self.tls_ctx.use_certificate_chain_file(file)
    }

    pub fn load_priv_key_from_pem_file(&mut self, file: &str) -> Result<()> {
        self.tls_ctx.use_privkey_file(file)
    }

    pub fn load_verify_locations_from_file(
        &mut self,
        file: &str,
    ) -> Result<()> {
        self.tls_ctx.load_verify_locations_from_file(file)
    }
}

// From quiche/src/tls/mod.rs
impl Context {
    pub fn use_certificate_chain_file(&mut self, file: &str) -> Result<()> {
        let cstr = ffi::CString::new(file).map_err(|_| Error::TlsFail)?;
        map_result(unsafe {
            SSL_CTX_use_certificate_chain_file(self.as_mut_ptr(), cstr.as_ptr())
        })
    }

    pub fn use_privkey_file(&mut self, file: &str) -> Result<()> {
        let cstr = ffi::CString::new(file).map_err(|_| Error::TlsFail)?;
        map_result(unsafe {
            SSL_CTX_use_PrivateKey_file(self.as_mut_ptr(), cstr.as_ptr(), 1)
        })
    }
}
```

### 7.2 Peer Verification

```rust
// From quiche/src/lib.rs
impl Config {
    pub fn verify_peer(&mut self, verify: bool) {
        self.tls_ctx.set_verify(verify);
    }
}

// From quiche/src/tls/mod.rs
impl Context {
    pub fn set_verify(&mut self, verify: bool) {
        // SSL_VERIFY_PEER = 0x01
        // SSL_VERIFY_NONE = 0x00
        let mode = i32::from(verify);
        unsafe {
            SSL_CTX_set_verify(self.as_mut_ptr(), mode, None);
        }
    }
}
```

### 7.3 Certificate Validation Callback

```rust
// Custom certificate validation
extern "C" fn verify_callback(
    preverify_ok: c_int,
    ctx: *mut X509_STORE_CTX,
) -> c_int {
    if preverify_ok == 0 {
        // Verification failed
        let err = X509_STORE_CTX_get_error(ctx);
        eprintln!("Certificate error: {}", err);
        return 0;
    }

    // Additional custom validation
    let cert = X509_STORE_CTX_get_current_cert(ctx);
    if !custom_verify(cert) {
        return 0;
    }

    1  // OK
}

fn custom_verify(cert: *mut X509) -> bool {
    // Check certificate properties
    // Validate against custom CA
    // Check revocation lists
    true
}
```

---

## Summary

### Key Takeaways

1. **TLS 1.3** - Simplified cipher suites, 1-RTT handshake, perfect forward secrecy
2. **Key derivation** - HKDF-based key schedule with separate epochs
3. **Key rotation** - KEY_PHASE bit triggers key update without renegotiation
4. **0-RTT** - Early data possible with replay attack considerations
5. **BoringSSL** - QUIC method provides callbacks for key management
6. **Certificates** - PEM loading, verification, custom validation callbacks

### Next Steps

Continue to [04-congestion-control-deep-dive.md](04-congestion-control-deep-dive.md) for:
- Loss detection algorithms
- Cubic and BBR2 congestion control
- Pacing and release timing
- HyStart++ slow-start exit

---

## Further Reading

- [RFC 8446 - TLS 1.3](https://www.rfc-editor.org/rfc/rfc8446.html)
- [RFC 9001 - Using TLS to Secure QUIC](https://www.rfc-editor.org/rfc/rfc9001.html)
- [BoringSSL QUIC Integration](https://github.com/google/boringssl/blob/master/include/openssl/quic.h)
- [quiche source - tls/mod.rs](quiche/src/tls/mod.rs)
- [quiche source - crypto/mod.rs](quiche/src/crypto/mod.rs)
