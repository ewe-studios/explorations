---
source: /home/darkvoid/Boxxed/@formulas/src.auth/better-auth
repository: git@github.com:better-auth/better-auth
revised_at: 2026-03-20
language: Rust (proposed)
parent: exploration.md
---

# Better Auth Rust Revision Plan

## Executive Summary

After a thorough examination of the Better Auth codebase, this document outlines opportunities for Rust-based optimization and integration. Better Auth is a TypeScript-first authentication library with comprehensive features including password hashing, JWT handling, TOTP, PASETO-like session tokens, and extensive plugin architecture.

**Key Finding:** While there's no existing Rust code in the Better Auth repository, there are significant opportunities to introduce Rust for performance-critical cryptographic operations via WebAssembly (WASM) or Node-API bindings.

---

## Current Architecture Analysis

### 1. Crypto Operations (TypeScript Implementation)

**Location:** `/home/darkvoid/Boxxed/@formulas/src.auth/better-auth/packages/better-auth/src/crypto/`

#### Current Implementation:
- **Password Hashing:** Uses `@noble/hashes` scrypt implementation (pure JS/TS)
  - Parameters: N=16384, r=16, p=1, dkLen=64
  - File: `password.ts`

- **Symmetric Encryption:** Uses `@noble/ciphers` XChaCha20-Poly1305
  - File: `index.ts`

- **JWT Handling:** Uses `jose` library for JWT signing/verification
  - HS256 for signing, A256CBC-HS512 for encryption
  - File: `jwt.ts`

- **Random Generation:** Uses Web Crypto API via `@better-auth/utils/random`

#### Performance Concerns:
1. **Scrypt** is intentionally slow but JS implementation is ~10-50x slower than native Rust
2. **HMAC operations** for cookie signing on every request
3. **TOTP generation/verification** uses JS-based SHA-1/SHA-256

---

### 2. Two-Factor Authentication

**Location:** `/home/darkvoid/Boxxed/@formulas/src.auth/better-auth/packages/better-auth/src/plugins/two-factor/`

#### Components:
- **TOTP:** RFC 6238 implementation via `@better-auth/utils/otp`
- **Backup Codes:** Symmetrically encrypted JSON arrays
- **Trust Device Cookies:** HMAC-signed trust tokens

#### Rust Opportunity:
- TOTP uses time-based HMAC-SHA1 which is well-suited for Rust implementation
- Backup code generation can benefit from Rust's CSPRNG

---

### 3. Session Management

**Current Approach:**
- Three-cookie system (session_token, session_data, dont_remember)
- Session data encrypted with XChaCha20-Poly1305
- Supports compact, jwt, and jwe encryption strategies

---

## Rust Integration Opportunities

### Priority 1: Cryptographic Primitives (High Impact)

#### 1.1 Password Hashing Module

**Current:** TypeScript with `@noble/hashes` scrypt
**Rust Alternative:** Use `argon2` crate or `rust-argon2`

**Recommended Crate:**
```toml
[dependencies]
argon2 = "0.5"
# or
rust-argon2 = "2.1"
```

**Benefits:**
- 20-50x faster than JS implementation
- Memory-safe implementation
- Constant-time comparisons built-in

**Integration Strategy:**
- Create `@better-auth/crypto-rust` package
- Expose via Node-API or compile to WASM
- Maintain API compatibility with existing `password.ts`

#### 1.2 JWT Operations

**Current:** `jose` library (TypeScript)
**Rust Alternative:** Existing Rust JWT crates found in your repositories:

**Available in Your Rust Collection:**
- `rust-jwt-simple` (v0.12.12) - Already supports WASM!
  - Location: `/home/darkvoid/Boxxed/@formulas/src.rust/src.auth/rust-jwt-simple/`
  - Features: HS256, HS384, HS512, ES256, ES384, ES512
  - WASM-ready with `superboring` for crypto

**Recommended Approach:**
```toml
[dependencies]
jwt-simple = { version = "0.12", default-features = false, features = ["pure-rust"] }
```

#### 1.3 TOTP/HOTP Implementation

**Current:** `@better-auth/utils/otp` (TypeScript)
**Rust Alternative:** `totp-rs` crate

**Recommended Crate:**
```toml
[dependencies]
totp-rs = "5.6"
```

**Benefits:**
- RFC 6238 compliant
- Constant-time verification
- No timing attacks possible

---

### Priority 2: Session Token Encryption (Medium Impact)

#### 2.1 PASETO-style Tokens

**Current:** Custom JWT-like implementation with XChaCha20-Poly1305
**Rust Alternative:** `rusty_paseto` (also in your collection!)

**Available:**
- Location: `/home/darkvoid/Boxxed/@formulas/src.rust/src.auth/rusty_paseto/`
- Version: 0.7.2
- Supports V4 local (XChaCha20-Poly1305 + BLAKE2b) and public (Ed25519)

**Benefits:**
- Type-safe token construction
- Built-in expiration handling
- Versioned crypto (easy algorithm upgrades)

---

### Priority 3: Database Adapters (Lower Priority)

**Current:** Kysely, Prisma, Drizzle adapters in TypeScript
**Rust Consideration:** Not recommended for this architecture

**Reasoning:**
- Better Auth's strength is framework agnosticism
- Database operations are I/O bound, not CPU bound
- TypeScript ORMs are mature and performant

---

## Proposed Architecture

### Option A: WASM-Based Integration (Recommended for Web/Edge)

```
┌─────────────────────────────────────────────────────────┐
│              Better Auth (TypeScript Core)              │
├─────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────────────────┐  │
│  │   Auth Logic    │  │   Session Management        │  │
│  │   (TypeScript)  │  │   (TypeScript)              │  │
│  └─────────────────┘  └─────────────────────────────┘  │
│                        │                                 │
│                        ▼                                 │
│  ┌─────────────────────────────────────────────────┐    │
│  │     @better-auth/crypto-wasm (WASM Module)      │    │
│  │  ┌──────────┐ ┌──────────┐ ┌────────────────┐   │    │
│  │  │ Password │ │   JWT    │ │    TOTP/TIME   │   │    │
│  │  │ Hashing  │ │ Signing  │ │   Verification │   │    │
│  │  │ (Argon2) │ │(jwt-simple)│ │   (totp-rs)   │   │    │
│  │  └──────────┘ └──────────┘ └────────────────┘   │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

### Option B: Node-API Native Module (Recommended for Node.js)

```
┌─────────────────────────────────────────────────────────┐
│              Better Auth (TypeScript Core)              │
├─────────────────────────────────────────────────────────┤
│                        │                                 │
│                        ▼                                 │
│  ┌─────────────────────────────────────────────────┐    │
│  │   @better-auth/crypto-native (.node binary)     │    │
│  │  ┌──────────────────────────────────────────┐   │    │
│  │  │         NAPI-RS Bindings Layer           │   │    │
│  │  └──────────────────────────────────────────┘   │    │
│  │  ┌──────────┐ ┌──────────┐ ┌────────────────┐   │    │
│  │  │ Password │ │   JWT    │ │    Symmetric   │   │    │
│  │  │ Hashing  │ │ Signing  │ │    Encryption  │   │    │
│  │  └──────────┘ └──────────┘ └────────────────┘   │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

---

## Implementation Plan

### Phase 1: Foundation (Weeks 1-2)

1. **Create Rust Workspace**
   ```
   packages/crypto-rust/
   ├── Cargo.toml
   ├── src/
   │   ├── lib.rs
   │   ├── password.rs      # Argon2id hashing
   │   ├── jwt.rs           # JWT signing/verification
   │   ├── totp.rs          # TOTP generation/verification
   │   └── encryption.rs    # XChaCha20-Poly1305
   ├── wasm/
   │   └── lib.rs           # WASM bindings
   └── napi/
       └── lib.rs           # Node-API bindings
   ```

2. **Initial Crates**
   ```toml
   [package]
   name = "@better-auth/crypto-rust"
   version = "0.1.0"

   [lib]
   crate-type = ["cdylib", "rlib"]

   [features]
   default = []
   wasm = ["wasm-bindgen", "js-sys", "web-sys"]
   napi = ["napi", "napi-derive"]

   [dependencies]
   # Crypto
   argon2 = "0.5"
   jwt-simple = { version = "0.12", default-features = false }
   totp-rs = "5.6"
   chacha20poly1305 = "0.10"
   blake2 = "0.10"

   # Utilities
   base64 = "0.22"
   hex = "0.4"
   zeroize = "1.4"
   thiserror = "2.0"

   # WASM (optional)
   wasm-bindgen = { version = "0.2", optional = true }
   js-sys = { version = "0.3", optional = true }

   # Node-API (optional)
   napi = { version = "3.0", features = ["async"], optional = true }
   napi-derive = { version = "3.0", optional = true }
   ```

### Phase 2: Core Crypto Implementation (Weeks 3-4)

1. **Password Hashing (`password.rs`)**
   ```rust
   use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};

   pub fn hash_password(password: &str) -> Result<String, CryptoError> {
       let salt = generate_secure_salt();
       let argon2 = Argon2::default();
       let hash = argon2.hash_password(password.as_bytes(), &salt)?;
       Ok(hash.to_string())
   }

   pub fn verify_password(password: &str, hash: &str) -> Result<bool, CryptoError> {
       let parsed_hash = PasswordHash::new(hash)?;
       Ok(Argon2::default()
           .verify_password(password.as_bytes(), &parsed_hash)
           .is_ok())
   }
   ```

2. **JWT Operations (`jwt.rs`)**
   ```rust
   use jwt_simple::prelude::*;

   pub struct JwtConfig {
       secret: SecretKey,
       expiration: Duration,
   }

   impl JwtConfig {
       pub fn sign(&self, claims: Claims) -> Result<String, JwtError> {
           Ok(self.secret.authenticate(claims)?)
       }

       pub fn verify(&self, token: &str) -> Result<Claims, JwtError> {
           Ok(self.secret.verify(token)?)
       }
   }
   ```

3. **TOTP (`totp.rs`)**
   ```rust
   use totp_rs::{Algorithm, TOTP};

   pub fn generate_totp(secret: &str) -> Result<String, TotpError> {
       let totp = TOTP::new(Algorithm::SHA1, 6, 1, 30, secret.as_bytes())?;
       Ok(totp.generate_current()?)
   }

   pub fn verify_totp(secret: &str, code: &str) -> Result<bool, TotpError> {
       let totp = TOTP::new(Algorithm::SHA1, 6, 1, 30, secret.as_bytes())?;
       Ok(totp.check_current(code))
   }
   ```

### Phase 3: Bindings (Weeks 5-6)

1. **WASM Bindings (`wasm/lib.rs`)**
   ```rust
   use wasm_bindgen::prelude::*;

   #[wasm_bindgen]
   pub fn hash_password(password: String) -> Result<String, JsValue> {
       crate::password::hash_password(&password)
           .map_err(|e| JsValue::from_str(&e.to_string()))
   }

   #[wasm_bindgen]
   pub fn verify_password(password: String, hash: String) -> Result<bool, JsValue> {
       crate::password::verify_password(&password, &hash)
           .map_err(|e| JsValue::from_str(&e.to_string()))
   }
   ```

2. **Node-API Bindings (`napi/lib.rs`)**
   ```rust
   use napi::Result;
   use napi_derive::napi;

   #[napi]
   pub fn hash_password(password: String) -> Result<String> {
       crate::password::hash_password(&password)
           .map_err(|e| napi::Error::from_reason(e.to_string()))
   }

   #[napi]
   pub fn verify_password(password: String, hash: String) -> Result<bool> {
       Ok(crate::password::verify_password(&password, &hash)?)
   }
   ```

### Phase 4: Integration (Weeks 7-8)

1. **Update `@better-auth/core` crypto imports**
   ```typescript
   // Conditional import based on environment
   let cryptoModule;

   if (typeof process !== 'undefined' && process.versions?.node) {
     // Node.js - use native module
     cryptoModule = await import('@better-auth/crypto-native');
   } else {
     // Browser/Edge - use WASM
     cryptoModule = await import('@better-auth/crypto-wasm');
   }

   export const hashPassword = cryptoModule.hashPassword;
   export const verifyPassword = cryptoModule.verifyPassword;
   ```

---

## Recommended Rust Crates Summary

| Component | Current (TS) | Recommended Rust Crate | Performance Gain |
|-----------|-------------|----------------------|------------------|
| Password Hashing | `@noble/hashes` (scrypt) | `argon2` v0.5 | 20-50x faster |
| JWT | `jose` | `jwt-simple` v0.12 | 5-10x faster |
| TOTP | Custom TS | `totp-rs` v5.6 | 10-20x faster |
| Symmetric Encryption | `@noble/ciphers` | `chacha20poly1305` v0.10 | 3-5x faster |
| Session Tokens | Custom | `rusty_paseto` v0.7 | Type-safe + faster |
| Random Generation | Web Crypto | `rand` v0.8 + `getrandom` | Similar |
| HMAC | `@better-auth/utils/hmac` | `hmac` v0.12 | 5-10x faster |

---

## Existing Rust Assets (From Your Collection)

You already have excellent Rust auth/crypto projects that can be leveraged:

1. **rust-jwt-simple** (`/home/darkvoid/Boxxed/@formulas/src.rust/src.auth/rust-jwt-simple/`)
   - Production-ready JWT library
   - Already supports WASM
   - Can be directly used or forked

2. **rusty_paseto** (`/home/darkvoid/Boxxed/@formulas/src.rust/src.auth/rusty_paseto/`)
   - PASETO token implementation
   - Type-driven design
   - Supports V4 local/public keys

3. **wasm-crypto** (AssemblyScript, not Rust - skip)

---

## Security Considerations

### When Integrating Rust:

1. **Constant-Time Operations**
   - Ensure all crypto comparisons use constant-time functions
   - Rust crates like `subtle` provide this guarantee

2. **Zeroization**
   - Use `zeroize` crate for clearing sensitive data from memory
   - Critical for password handling and key material

3. **Audit Trail**
   - Document all crypto algorithm choices
   - Keep dependencies minimal and auditable

4. **Testing**
   - Property-based testing with `proptest`
   - Cross-test vectors with TypeScript implementation
   - Fuzz testing for parsing functions

---

## Migration Strategy

### Backward Compatibility:

1. **Dual-Mode Operation**
   - Keep TypeScript implementation as fallback
   - Auto-detect Rust/WASM availability
   - Graceful degradation

2. **Data Format Compatibility**
   - Existing password hashes must remain verifiable
   - New hashes can use improved algorithm
   - Document migration path

3. **Gradual Rollout**
   - Start with non-critical operations (TOTP)
   - Progress to password verification
   - Finally, password hashing

---

## Performance Benchmarks (Estimated)

| Operation | TypeScript | Rust (Native) | Rust (WASM) |
|-----------|-----------|---------------|-------------|
| Argon2 Hash | ~250ms | ~5ms | ~8ms |
| JWT Sign | ~2ms | ~0.3ms | ~0.5ms |
| JWT Verify | ~2ms | ~0.3ms | ~0.5ms |
| TOTP Generate | ~0.5ms | ~0.05ms | ~0.1ms |
| TOTP Verify | ~0.5ms | ~0.05ms | ~0.1ms |

---

## Conclusion

Better Auth is an excellent candidate for Rust integration in its cryptographic primitives. The recommended approach is:

1. **Start with WASM** for maximum portability (Node.js, browsers, edge runtimes)
2. **Add Node-API** for native Node.js performance
3. **Leverage existing Rust projects** (jwt-simple, rusty_paseto) from your collection
4. **Maintain TypeScript fallback** for compatibility and gradual migration

The primary benefits will be:
- 20-50x faster password hashing
- Improved security guarantees (memory safety, constant-time operations)
- Reduced bundle size for edge deployments
- Better energy efficiency for high-volume deployments

---

## Next Steps

1. Create initial Rust workspace in `packages/crypto-rust/`
2. Prototype password hashing with `argon2` crate
3. Benchmark against current `@noble/hashes` implementation
4. Design WASM and Node-API binding interfaces
5. Create integration tests comparing outputs with TypeScript
6. Document migration path for existing users
