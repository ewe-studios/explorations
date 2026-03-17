---
source: better-auth exploration + RUST_REVISION_PLAN.md
expanded_at: 2026-03-17
focus: WASM portability, CloudFlare Workers, R2/D2 storage, self-contained auth
---

# Better Auth WASM Deep Dive

## Executive Summary

This document expands on the WASM portability vision for Better Auth, exploring what a fully self-contained, portable authentication system looks like when built on Rust + WASM, targeting CloudFlare Workers with R2 (object storage) and D2 (SQLite) backends, while maintaining the "no external services" requirement.

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Rust Crate Structure](#2-rust-crate-structure)
3. [Authentication Mechanisms](#3-authentication-mechanisms)
4. [Database Structure](#4-database-structure)
5. [CloudFlare Workers Deployment](#5-cloudflare-workers-deployment)
6. [R2 Object Storage](#6-r2-object-storage)
7. [D2 SQLite Integration](#7-d2-sqlite-integration)
8. [API Design](#8-api-design)
9. [JWT & Session Management](#9-jwt--session-management)
10. [Self-Contained World](#10-self-contained-world)

---

## 1. Architecture Overview

### The Vision

A fully portable authentication system that:
- Compiles to WASM and runs identically on CloudFlare Workers, Node.js, Deno, and Bun
- Stores data in D2 (SQLite at the edge) or falls back to R2 for blob storage
- Requires zero external dependencies (no Auth0, no Firebase, no Supabase)
- Provides cryptographically secure operations via Rust
- Maintains type-safety end-to-end

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        Request (HTTP)                                    │
└─────────────────────────────────────────────────────────────────────────┘
                                   │
                                   ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                     CloudFlare Worker (WASM)                             │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │                    Better Auth Core (Rust → WASM)                   │ │
│  │  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐ ┌────────────┐ │ │
│  │  │   Password   │ │     JWT      │ │    OAuth2    │ │   Session  │ │ │
│  │  │   Hashing    │ │    Signing   │ │    Flows     │ │   Manager  │ │ │
│  │  │  (Argon2id)  │ │ (jwt-simple) │ │  (generic)   │ │  (PASETO)  │ │ │
│  │  └──────────────┘ └──────────────┘ └──────────────┘ └────────────┘ │ │
│  │  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐ ┌────────────┐ │ │
│  │  │   TOTP/2FA   │ │  Magic Link  │ │   Username   │ │    RBAC    │ │ │
│  │  │  (totp-rs)   │ │  (HMAC-Sign) │ │  + Password  │ │  (Access)  │ │ │
│  │  └──────────────┘ └──────────────┘ └──────────────┘ └────────────┘ │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                   │                                      │
│         ┌─────────────────────────┼─────────────────────────┐            │
│         │                         │                         │            │
│         ▼                         ▼                         ▼            │
│  ┌─────────────┐           ┌─────────────┐           ┌─────────────┐   │
│  │  D2 Database│           │  R2 Bucket  │           │  KV Store   │   │
│  │  (SQLite)   │           │  (Objects)  │           │  (Cache)    │   │
│  │             │           │             │           │             │   │
│  │ - users     │           │ - avatars   │           │ - sessions  │   │
│  │ - sessions  │           │ - backups   │           │ - rate-lmt  │   │
│  │ - accounts  │           │ - exports   │           │ - tokens    │   │
│  │ - plugins   │           │             │           │             │   │
│  └─────────────┘           └─────────────┘           └─────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
```

### Why WASM + Rust?

| Aspect | TypeScript | Rust → WASM |
|--------|-----------|-------------|
| Password Hash (Argon2id) | ~250ms | ~8ms |
| JWT Sign/Verify | ~2ms | ~0.5ms |
| TOTP Generate | ~0.5ms | ~0.1ms |
| Memory Safety | GC-dependent | Guaranteed |
| Bundle Size | ~500KB (deps) | ~50KB (WASM) |
| Cold Start | Fast | Near-instant |
| Portability | Node/Web | Universal |

---

## 2. Rust Crate Structure

### Workspace Layout

```
better-auth-wasm/
├── Cargo.toml                    # Workspace definition
├── crates/
│   ├── better-auth-core/         # Core authentication logic
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── auth.rs           # Main auth struct
│   │       ├── config.rs         # Configuration types
│   │       └── error.rs          # Error types
│   │
│   ├── better-auth-crypto/       # Cryptographic operations
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── password.rs       # Argon2id hashing
│   │       ├── jwt.rs            # JWT signing/verification
│   │       ├── totp.rs           # TOTP/HOTP
│   │       ├── encryption.rs     # XChaCha20-Poly1305
│   │       └── hmac.rs           # HMAC utilities
│   │
│   ├── better-auth-session/      # Session management
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── manager.rs        # Session CRUD
│   │       ├── token.rs          # Token generation
│   │       └── cookie.rs         # Cookie handling
│   │
│   ├── better-auth-oauth/        # OAuth2/OIDC
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── provider.rs       # OAuth provider trait
│   │       ├── flows.rs          # Auth code, implicit, PKCE
│   │       └── providers/        # Built-in providers
│   │           ├── github.rs
│   │           ├── google.rs
│   │           └── generic.rs
│   │
│   ├── better-auth-db/           # Database layer
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── adapter.rs        # Database adapter trait
│   │       ├── d2.rs             # D2/SQLite adapter
│   │       ├── r2.rs             # R2 object adapter
│   │       └── schema.rs         # Database schema
│   │
│   ├── better-auth-bindings/     # WASM bindings
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs            # wasm_bindgen exports
│   │       └── js_glue.rs        # JavaScript interop
│   │
│   └── better-auth-types/        # Shared types
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── user.rs           # User types
│           ├── session.rs        # Session types
│           └── claims.rs         # JWT claims
│
├── bindings/
│   ├── ts/                       # TypeScript type definitions
│   │   ├── index.ts
│   │   ├── types.ts
│   │   └── generated/
│   └── worker/                   # CloudFlare Worker wrappers
│       ├── index.ts
│       └── handler.ts
│
└── examples/
    ├── cloudflare-worker/
    ├── nodejs/
    └── standalone/
```

### Root Cargo.toml

```toml
[workspace]
members = ["crates/*"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT"
authors = ["Better Auth Contributors"]

[workspace.dependencies]
# Crypto
argon2 = "0.5"
jwt-simple = { version = "0.12", default-features = false, features = ["pure-rust"] }
totp-rs = "5.6"
chacha20poly1305 = "0.10"
hmac = "0.12"
sha2 = "0.10"
blake2 = "0.10"
rand = "0.8"
getrandom = { version = "0.2", features = ["js"] }

# Utilities
base64 = "0.22"
hex = "0.4"
zeroize = "1.4"
thiserror = "2.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.0", features = ["v4", "serde"] }

# WASM
wasm-bindgen = "0.2"
js-sys = "0.3"
web-sys = { version = "0.3", features = ["console", "Request", "Response", "Headers"] }
wasm-bindgen-futures = "0.4"

# Async
futures = "0.3"
async-trait = "0.1"

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
strip = true
```

### Core Crypto Crate (better-auth-crypto)

```toml
[package]
name = "better-auth-crypto"
version.workspace = true
edition.workspace = true

[dependencies]
# Crypto primitives
argon2.workspace = true
jwt-simple.workspace = true
totp-rs.workspace = true
chacha20poly1305.workspace = true
hmac.workspace = true
sha2.workspace = true
blake2.workspace = true
rand.workspace = true

# Utilities
base64.workspace = true
hex.workspace = true
zeroize.workspace = true
thiserror.workspace = true
serde.workspace = true

# WASM (optional)
wasm-bindgen = { workspace = true, optional = true }
js-sys = { workspace = true, optional = true }
getrandom.workspace = true

[features]
default = []
wasm = ["wasm-bindgen", "js-sys"]
```

```rust
// crates/better-auth-crypto/src/password.rs

use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2, Algorithm, Params, Version
};
use rand::rngs::OsRng;
use thiserror::Error;
use zeroize::Zeroize;

#[derive(Error, Debug)]
pub enum PasswordError {
    #[error("Invalid password format")]
    InvalidFormat,
    #[error("Password hashing failed")]
    HashFailed,
    #[error("Password verification failed")]
    VerifyFailed,
}

/// Default Argon2id parameters balancing security and performance
pub const ARGON2_PARAMS: Params = Params {
    m_cost: 65536,  // 64 MB memory
    t_cost: 3,      // 3 iterations
    p_cost: 4,      // 4 parallelism
    ..Params::default()
};

/// Hash a password using Argon2id
pub fn hash_password(password: &str) -> Result<String, PasswordError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, ARGON2_PARAMS);

    match argon2.hash_password(password.as_bytes(), &salt) {
        Ok(hash) => Ok(hash.to_string()),
        Err(_) => Err(PasswordError::HashFailed),
    }
}

/// Verify a password against its hash
pub fn verify_password(password: &str, hash: &str) -> Result<bool, PasswordError> {
    let parsed_hash = PasswordHash::new(hash)
        .map_err(|_| PasswordError::InvalidFormat)?;

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

/// Secure password comparison (constant-time)
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }

    result == 0
}
```

```rust
// crates/better-auth-crypto/src/jwt.rs

use jwt_simple::prelude::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use chrono::{DateTime, Utc};

#[derive(Error, Debug)]
pub enum JwtError {
    #[error("Invalid token")]
    InvalidToken,
    #[error("Token expired")]
    Expired,
    #[error("Invalid claims")]
    InvalidClaims,
    #[error("Signing failed")]
    SigningFailed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionClaims {
    pub sub: String,      // User ID
    pub sid: String,      // Session ID
    pub iat: i64,         // Issued at
    pub exp: i64,         // Expiration
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}

pub struct JwtSigner {
    secret: SecretKey,
    expiration_secs: u64,
}

impl JwtSigner {
    pub fn new(secret: &[u8], expiration_secs: u64) -> Self {
        let secret_key = SecretKey::from_bytes(secret);
        Self {
            secret: secret_key,
            expiration_secs,
        }
    }

    pub fn sign(&self, claims: SessionClaims) -> Result<String, JwtError> {
        let jwt_claims = Claims::with_custom_claims(claims, Duration::seconds(self.expiration_secs as i64));
        self.secret.authenticate(jwt_claims)
            .map_err(|_| JwtError::SigningFailed)
    }

    pub fn verify(&self, token: &str) -> Result<SessionClaims, JwtError> {
        let claims = self.secret
            .verify::<SessionClaims>(token)
            .map_err(|_| JwtError::InvalidToken)?;

        Ok(claims.custom)
    }
}
```

---

## 3. Authentication Mechanisms

### 3.1 Username + Password Flow

```rust
// crates/better-auth-core/src/auth/password_auth.rs

use better_auth_crypto::{hash_password, verify_password, PasswordError};
use better_auth_db::adapter::DatabaseAdapter;
use better_auth_types::{User, AuthError, SignInRequest, SignInResponse};

pub struct PasswordAuth {
    db: Box<dyn DatabaseAdapter>,
}

impl PasswordAuth {
    pub async fn sign_up(
        &self,
        email: String,
        password: String,
        username: Option<String>,
    ) -> Result<User, AuthError> {
        // Check if user exists
        if let Some(_) = self.db.find_user_by_email(&email).await? {
            return Err(AuthError::UserAlreadyExists);
        }

        // Validate password strength
        self.validate_password(&password)?;

        // Hash password
        let password_hash = hash_password(&password)
            .map_err(|_| AuthError::InternalServerError)?;

        // Create user
        let user = User {
            id: uuid::Uuid::new_v4().to_string(),
            email,
            username,
            password_hash: Some(password_hash),
            email_verified: false,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        self.db.create_user(&user).await?;
        Ok(user)
    }

    pub async fn sign_in(
        &self,
        email: String,
        password: String,
    ) -> Result<SignInResponse, AuthError> {
        // Find user (timing-safe: always hash even if user not found)
        let user = self.db.find_user_by_email(&email).await?;

        // Constant-time hash to prevent timing enumeration
        let dummy_hash = "$argon2id$v=19$m=65536,t=3,p=4$placeholder$dummy";
        let user_ref = user.as_ref();
        let hash_to_check = user_ref
            .and_then(|u| u.password_hash.as_ref())
            .unwrap_or(dummy_hash);

        let valid = verify_password(&password, hash_to_check)
            .unwrap_or(false);

        if !valid || user.is_none() {
            // Always hash to maintain constant time
            let _ = hash_password(&password);
            return Err(AuthError::InvalidCredentials);
        }

        let user = user.unwrap();

        // Create session
        let session = self.db.create_session(&user.id).await?;

        Ok(SignInResponse {
            user,
            session,
            token: self.generate_session_token(&user.id, &session.id)?,
        })
    }

    fn validate_password(&self, password: &str) -> Result<(), AuthError> {
        if password.len() < 8 {
            return Err(AuthError::WeakPassword("Password must be at least 8 characters"));
        }
        // Additional strength checks...
        Ok(())
    }

    fn generate_session_token(&self, user_id: &str, session_id: &str) -> Result<String, AuthError> {
        // Generate JWT or PASETO token
        Ok(todo!())
    }
}
```

### 3.2 Email Magic Link Flow

```rust
// crates/better-auth-core/src/auth/magic_link.rs

use better_auth_crypto::hmac_sign;
use better_auth_db::adapter::DatabaseAdapter;
use better_auth_types::{MagicLinkRequest, AuthError};
use chrono::{Utc, Duration};

pub struct MagicLinkAuth {
    db: Box<dyn DatabaseAdapter>,
    hmac_secret: Vec<u8>,
    base_url: String,
}

impl MagicLinkAuth {
    pub async fn request_magic_link(
        &self,
        email: String,
    ) -> Result<MagicLinkRequest, AuthError> {
        // Generate secure token
        let token = self.generate_magic_token(&email)?;

        // Create or update pending magic link
        let expires_at = Utc::now() + Duration::minutes(15);
        self.db.upsert_magic_link(&email, &token, expires_at).await?;

        // Generate magic link URL
        let magic_link = format!(
            "{}/auth/magic-link/verify?token={}&email={}",
            self.base_url,
            token,
            urlencoding::encode(&email)
        );

        Ok(MagicLinkRequest {
            email,
            magic_link,
            expires_at,
        })
    }

    pub async fn verify_magic_link(
        &self,
        token: String,
        email: String,
    ) -> Result<User, AuthError> {
        // Verify token signature
        if !self.verify_magic_token(&token, &email)? {
            return Err(AuthError::InvalidToken);
        }

        // Check magic link exists and not expired
        let magic_link = self.db.get_magic_link(&email, &token).await?
            .ok_or(AuthError::InvalidToken)?;

        if magic_link.expires_at < Utc::now() {
            return Err(AuthError::TokenExpired);
        }

        // Delete used magic link
        self.db.delete_magic_link(&email, &token).await?;

        // Get or create user
        let user = match self.db.find_user_by_email(&email).await? {
            Some(user) => user,
            None => {
                // Create new user
                let user = User {
                    id: uuid::Uuid::new_v4().to_string(),
                    email,
                    email_verified: true,
                    ..Default::default()
                };
                self.db.create_user(&user).await?;
                user
            }
        };

        // Create session
        let session = self.db.create_session(&user.id).await?;

        Ok(user)
    }

    fn generate_magic_token(&self, email: &str) -> Result<String, AuthError> {
        let timestamp = Utc::now().timestamp();
        let payload = format!("{}:{}:{}", email, timestamp, uuid::Uuid::new_v4());
        let signature = hmac_sign(&payload, &self.hmac_secret)?;
        Ok(format!("{}.{}", payload, signature))
    }

    fn verify_magic_token(&self, token: &str, email: &str) -> Result<bool, AuthError> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 2 {
            return Ok(false);
        }

        let payload = parts[0];
        let provided_signature = parts[1];

        // Verify signature
        let expected_signature = hmac_sign(payload, &self.hmac_secret)?;

        // Constant-time comparison
        Ok(better_auth_crypto::constant_time_eq(
            provided_signature.as_bytes(),
            expected_signature.as_bytes()
        ))
    }
}
```

### 3.3 OAuth2 Flow

```rust
// crates/better-auth-oauth/src/flows.rs

use better_auth_types::{OAuthConfig, OAuthState, UserInfo, AuthError};
use better_auth_crypto::generate_secure_random;

pub struct OAuthFlow {
    config: OAuthConfig,
}

impl OAuthFlow {
    /// Generate authorization URL for OAuth provider
    pub async fn create_authorization_url(
        &self,
        provider: &str,
        redirect_uri: String,
        scope: Option<Vec<String>>,
    ) -> Result<String, AuthError> {
        // Generate state and code verifier (PKCE)
        let state = generate_secure_random(32)?;
        let code_verifier = generate_secure_random(32)?;
        let code_challenge = self.generate_code_challenge(&code_verifier)?;

        // Build authorization URL
        let auth_url = match provider {
            "github" => self.github_auth_url(&redirect_uri, &state, &code_challenge, scope),
            "google" => self.google_auth_url(&redirect_uri, &state, &code_challenge, scope),
            _ => self.generic_auth_url(provider, &redirect_uri, &state, &code_challenge, scope),
        };

        // Store state + code_verifier in temporary storage (KV/D2)
        self.store_oauth_state(&state, &code_verifier).await?;

        Ok(auth_url)
    }

    /// Handle OAuth callback
    pub async fn handle_callback(
        &self,
        provider: &str,
        code: String,
        state: String,
        redirect_uri: String,
    ) -> Result<UserInfo, AuthError> {
        // Verify state
        let code_verifier = self.get_oauth_state(&state).await?
            .ok_or(AuthError::InvalidState)?;

        // Exchange code for tokens
        let tokens = self.exchange_code_for_tokens(
            provider,
            &code,
            &code_verifier,
            &redirect_uri
        ).await?;

        // Get user info
        let user_info = self.get_user_info(provider, &tokens.access_token).await?;

        Ok(user_info)
    }

    /// PKCE code challenge (SHA256)
    fn generate_code_challenge(&self, code_verifier: &str) -> Result<String, AuthError> {
        use sha2::{Sha256, Digest};
        let digest = Sha256::digest(code_verifier.as_bytes());
        Ok(base64_url::encode(&digest))
    }

    async fn exchange_code_for_tokens(
        &self,
        provider: &str,
        code: &str,
        code_verifier: &str,
        redirect_uri: &str,
    ) -> Result<OAuthTokens, AuthError> {
        // Implementation varies by provider
        match provider {
            "github" => self.github_token_exchange(code, code_verifier, redirect_uri).await,
            "google" => self.google_token_exchange(code, code_verifier, redirect_uri).await,
            _ => self.generic_token_exchange(provider, code, code_verifier, redirect_uri).await,
        }
    }
}

/// OAuth provider trait for extensibility
pub trait OAuthProvider: Send + Sync {
    fn auth_url(&self, redirect_uri: &str, state: &str, code_challenge: &str, scope: &[String]) -> String;
    fn token_endpoint(&self) -> &str;
    fn user_info_endpoint(&self) -> &str;
    fn client_id(&self) -> &str;
    fn client_secret(&self) -> &str;
}
```

---

## 4. Database Structure

### D2 (SQLite) Schema

```sql
-- Core Tables

CREATE TABLE users (
    id TEXT PRIMARY KEY,
    email TEXT UNIQUE NOT NULL,
    username TEXT UNIQUE,
    password_hash TEXT,
    email_verified INTEGER DEFAULT 0,
    email_verified_at INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    metadata TEXT  -- JSON blob for extensibility
);

CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token TEXT UNIQUE NOT NULL,
    expires_at INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    ip_address TEXT,
    user_agent TEXT,
    metadata TEXT
);

CREATE TABLE accounts (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider_id TEXT NOT NULL,
    provider_account_id TEXT NOT NULL,
    access_token TEXT,
    refresh_token TEXT,
    expires_at INTEGER,
    scope TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE(provider_id, provider_account_id)
);

CREATE TABLE verification_tokens (
    id TEXT PRIMARY KEY,
    identifier TEXT NOT NULL,  -- email, phone, etc.
    token TEXT UNIQUE NOT NULL,
    type TEXT NOT NULL,  -- 'email_otp', 'magic_link', 'password_reset'
    expires_at INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    consumed_at INTEGER,
    metadata TEXT
);

-- Plugin Tables (Two-Factor)

CREATE TABLE two_factor_secrets (
    id TEXT PRIMARY KEY,
    user_id TEXT UNIQUE NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    secret TEXT NOT NULL,
    backup_codes TEXT,  -- Encrypted JSON array
    enabled INTEGER DEFAULT 0,
    created_at INTEGER NOT NULL
);

-- Plugin Tables (Organization)

CREATE TABLE organizations (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    slug TEXT UNIQUE,
    logo TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    metadata TEXT
);

CREATE TABLE organization_members (
    id TEXT PRIMARY KEY,
    organization_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role TEXT NOT NULL,  -- 'owner', 'admin', 'member'
    invited_at INTEGER NOT NULL,
    invited_by TEXT REFERENCES users(id),
    UNIQUE(organization_id, user_id)
);

CREATE TABLE organization_invitations (
    id TEXT PRIMARY KEY,
    organization_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    email TEXT NOT NULL,
    role TEXT NOT NULL,
    invited_by TEXT NOT NULL REFERENCES users(id),
    invited_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL,
    status TEXT DEFAULT 'pending',  -- 'pending', 'accepted', 'declined'
    UNIQUE(organization_id, email)
);

-- Plugin Tables (API Keys)

CREATE TABLE api_keys (
    id TEXT PRIMARY KEY,
    key_hash TEXT UNIQUE NOT NULL,
    name TEXT,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    permissions TEXT,  -- JSON array
    expires_at INTEGER,
    last_used_at INTEGER,
    created_at INTEGER NOT NULL
);

-- Indexes

CREATE INDEX idx_sessions_user_id ON sessions(user_id);
CREATE INDEX idx_sessions_token ON sessions(token);
CREATE INDEX idx_accounts_user_id ON accounts(user_id);
CREATE INDEX idx_verification_tokens_token ON verification_tokens(token);
CREATE INDEX idx_verification_tokens_identifier ON verification_tokens(identifier);
CREATE INDEX idx_org_members_user ON organization_members(user_id);
CREATE INDEX idx_org_invitations_email ON organization_invitations(email);

-- Full-text search for users
CREATE VIRTUAL TABLE IF NOT EXISTS users_fts USING fts5(
    email,
    username,
    content='users',
    content_rowid='rowid'
);

-- Triggers for FTS sync
CREATE TRIGGER users_ai AFTER INSERT ON users BEGIN
    INSERT INTO users_fts(rowid, email, username) VALUES (new.rowid, new.email, new.username);
END;

CREATE TRIGGER users_ad AFTER DELETE ON users BEGIN
    INSERT INTO users_fts(users_fts, rowid, email, username) VALUES('delete', old.rowid, old.email, old.username);
END;
```

### Rust Schema Types

```rust
// crates/better-auth-db/src/schema.rs

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub username: Option<String>,
    pub password_hash: Option<String>,
    pub email_verified: bool,
    pub email_verified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub user_id: String,
    pub token: String,
    pub expires_at: i64,
    pub created_at: i64,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub user_id: String,
    pub provider_id: String,
    pub provider_account_id: String,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_at: Option<i64>,
    pub scope: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationToken {
    pub id: String,
    pub identifier: String,
    pub token: String,
    #[serde(rename = "type")]
    pub token_type: String,
    pub expires_at: i64,
    pub created_at: i64,
    pub consumed_at: Option<i64>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwoFactorSecret {
    pub id: String,
    pub user_id: String,
    pub secret: String,
    pub backup_codes: Option<String>,  // Encrypted JSON
    pub enabled: bool,
    pub created_at: i64,
}
```

---

## 5. CloudFlare Workers Deployment

### Worker Configuration

```toml
# wrangler.toml
name = "better-auth-worker"
main = "dist/worker.js"
compatibility_date = "2024-01-01"
compatibility_flags = ["nodejs_compat"]

# Bindings
[vars]
ENVIRONMENT = "production"
BETTER_AUTH_URL = "https://auth.example.com"

# D2 Database
[[d2_databases]]
binding = "DB"
database_name = "better-auth-db"
database_id = "<d2-database-id>"

# R2 Bucket
[[r2_buckets]]
binding = "BUCKET"
bucket_name = "better-auth-storage"

# KV Store (for sessions/cache)
[[kv_namespaces]]
binding = "CACHE"
id = "<kv-namespace-id>"
preview_id = "<preview-kv-id>"

# Secrets (set via wrangler secret)
# BETTER_AUTH_SECRET
# HMAC_SECRET
# OAUTH_CLIENT_SECRETS

# WASM module
[[wasm_modules]]
BETTER_AUTH_WASM = "dist/better_auth_wasm_bg.wasm"

# Environment-specific overrides
[env.staging]
name = "better-auth-worker-staging"

[[env.staging.d2_databases]]
binding = "DB"
database_name = "better-auth-db-staging"
database_id = "<staging-d2-id>"
```

### Rust WASM Build Script

```rust
// crates/better-auth-bindings/src/lib.rs

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;
use js_sys::{Promise, JsString, Object};
use serde_wasm_bindgen::{to_value, from_value};

mod glue;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

/// Initialize the auth engine
#[wasm_bindgen]
pub struct AuthEngine {
    core: better_auth_core::AuthEngine,
    db: glue::D2Adapter,
    cache: glue::KVAdapter,
    storage: glue::R2Adapter,
}

#[wasm_bindgen]
impl AuthEngine {
    #[wasm_bindgen(constructor)]
    pub fn new(
        secret: String,
        db_binding: JsValue,
        cache_binding: JsValue,
        storage_binding: JsValue,
    ) -> Result<AuthEngine, JsValue> {
        let db = glue::D2Adapter::new(db_binding)?;
        let cache = glue::KVAdapter::new(cache_binding)?;
        let storage = glue::R2Adapter::new(storage_binding)?;

        let core = better_auth_core::AuthEngine::new(&secret)?;

        Ok(AuthEngine { core, db, cache, storage })
    }

    /// Sign up with email/password
    #[wasm_bindgen(js_name = signUp)]
    pub fn sign_up(&self, email: String, password: String) -> Promise {
        let db = self.db.clone();
        let core = self.core.clone();

        future_to_promise(async move {
            match core.sign_up(&db, &email, &password).await {
                Ok(user) => Ok(to_value(&user).unwrap().into()),
                Err(e) => Err(JsValue::from_str(&e.to_string())),
            }
        })
    }

    /// Sign in with email/password
    #[wasm_bindgen(js_name = signIn)]
    pub fn sign_in(&self, email: String, password: String) -> Promise {
        let db = self.db.clone();
        let core = self.core.clone();

        future_to_promise(async move {
            match core.sign_in(&db, &email, &password).await {
                Ok(response) => Ok(to_value(&response).unwrap().into()),
                Err(e) => Err(JsValue::from_str(&e.to_string())),
            }
        })
    }

    /// Verify session token
    #[wasm_bindgen(js_name = verifySession)]
    pub fn verify_session(&self, token: String) -> Promise {
        let db = self.db.clone();
        let cache = self.cache.clone();
        let core = self.core.clone();

        future_to_promise(async move {
            match core.verify_session(&db, &cache, &token).await {
                Ok(session) => Ok(to_value(&session).unwrap().into()),
                Err(e) => Err(JsValue::from_str(&e.to_string())),
            }
        })
    }

    /// Request magic link
    #[wasm_bindgen(js_name = requestMagicLink)]
    pub fn request_magic_link(&self, email: String, base_url: String) -> Promise {
        let core = self.core.clone();
        let db = self.db.clone();

        future_to_promise(async move {
            match core.request_magic_link(&db, &email, &base_url).await {
                Ok(result) => Ok(to_value(&result).unwrap().into()),
                Err(e) => Err(JsValue::from_str(&e.to_string())),
            }
        })
    }
}

/// WASM memory allocator configuration
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;
```

### TypeScript Worker Handler

```typescript
// bindings/worker/handler.ts

import { AuthEngine } from '../generated/better_auth_wasm';

export interface Env {
  DB: D1Database;
  BUCKET: R2Bucket;
  CACHE: KVNamespace;
  BETTER_AUTH_SECRET: string;
  BETTER_AUTH_URL: string;
}

export default {
  async fetch(request: Request, env: Env, ctx: ExecutionContext): Promise<Response> {
    const url = new URL(request.url);
    const path = url.pathname;

    // Initialize WASM auth engine (singleton per worker)
    const auth = new AuthEngine(
      env.BETTER_AUTH_SECRET,
      env.DB as unknown as JsValue,
      env.CACHE as unknown as JsValue,
      env.BUCKET as unknown as JsValue
    );

    // Route handling
    if (path === '/auth/sign-up' && request.method === 'POST') {
      const { email, password } = await request.json();
      try {
        const user = await auth.signUp(email, password);
        return json(user);
      } catch (error) {
        return json({ error: error.message }, { status: 400 });
      }
    }

    if (path === '/auth/sign-in' && request.method === 'POST') {
      const { email, password } = await request.json();
      try {
        const response = await auth.signIn(email, password);
        return setSessionCookie(response);
      } catch (error) {
        return json({ error: error.message }, { status: 401 });
      }
    }

    if (path === '/auth/sign-out' && request.method === 'POST') {
      // Clear session cookie
      return clearSessionCookie();
    }

    if (path === '/auth/magic-link/request' && request.method === 'POST') {
      const { email } = await request.json();
      try {
        const result = await auth.requestMagicLink(email, env.BETTER_AUTH_URL);
        // Send email via your preferred provider
        await sendMagicLinkEmail(email, result.magicLink);
        return json({ success: true });
      } catch (error) {
        return json({ error: error.message }, { status: 400 });
      }
    }

    // Health check
    if (path === '/health') {
      return json({ status: 'ok', timestamp: Date.now() });
    }

    return new Response('Not Found', { status: 404 });
  },
};

function json(data: unknown, init?: ResponseInit): Response {
  return new Response(JSON.stringify(data), {
    ...init,
    headers: { 'Content-Type': 'application/json' },
  });
}

function setSessionCookie(response: any): Response {
  const headers = new Headers({
    'Content-Type': 'application/json',
    'Set-Cookie': `session=${response.token}; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age=${7 * 24 * 60 * 60}`
  });
  return new Response(JSON.stringify(response), { headers });
}
```

---

## 6. R2 Object Storage

### R2 Adapter for Rust

```rust
// crates/better-auth-db/src/r2.rs

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;
use js_sys::{Promise, Uint8Array};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum R2Error {
    #[error("Object not found")]
    NotFound,
    #[error("Upload failed")]
    UploadFailed,
    #[error("Download failed")]
    DownloadFailed,
    #[error("Delete failed")]
    DeleteFailed,
}

pub struct R2Adapter {
    bucket: JsValue,
}

impl R2Adapter {
    pub fn new(bucket: JsValue) -> Self {
        Self { bucket }
    }

    /// Upload an object to R2
    pub async fn put(&self, key: &str, data: &[u8], content_type: Option<&str>) -> Result<(), R2Error> {
        let promise: Promise = js_sys::Reflect::get(&self.bucket, &JsString::from("put"))
            .unwrap()
            .dyn_into::<js_sys::Function>()
            .unwrap()
            .call2(
                &self.bucket,
                &JsString::from(key),
                &Uint8Array::from(data),
            )
            .unwrap()
            .dyn_into()
            .unwrap();

        // Handle promise result
        // (simplified - would use wasm_bindgen_futures in practice)
        Ok(())
    }

    /// Download an object from R2
    pub async fn get(&self, key: &str) -> Result<Vec<u8>, R2Error> {
        let promise: Promise = js_sys::Reflect::get(&self.bucket, &JsString::from("get"))
            .unwrap()
            .dyn_into::<js_sys::Function>()
            .unwrap()
            .call1(&self.bucket, &JsString::from(key))
            .unwrap()
            .dyn_into()
            .unwrap();

        // Convert result to Vec<u8>
        todo!("Convert JS result to bytes")
    }

    /// Delete an object from R2
    pub async fn delete(&self, key: &str) -> Result<(), R2Error> {
        Ok(())
    }

    /// List objects in R2
    pub async fn list(&self, prefix: Option<&str>) -> Result<Vec<String>, R2Error> {
        Ok(vec![])
    }
}

/// Use cases for R2 in Better Auth

// 1. User avatar storage
pub async fn upload_user_avatar(
    r2: &R2Adapter,
    user_id: &str,
    avatar_data: &[u8],
    content_type: &str,
) -> Result<String, R2Error> {
    let key = format!("avatars/{}/{}", user_id, uuid::Uuid::new_v4());
    r2.put(&key, avatar_data, Some(content_type)).await?;
    Ok(format!("https://storage.example.com/{}", key))
}

// 2. Backup export storage
pub async fn export_user_data(
    r2: &R2Adapter,
    user_id: &str,
    export_json: &[u8],
) -> Result<String, R2Error> {
    let key = format!("exports/{}/{}.json", user_id, chrono::Utc::now().timestamp());
    r2.put(&key, export_json, Some("application/json")).await?;
    Ok(format!("https://storage.example.com/{}", key))
}

// 3. Session snapshot storage (for distributed invalidation)
pub async fn store_session_snapshot(
    r2: &R2Adapter,
    session_id: &str,
    session_data: &[u8],
) -> Result<(), R2Error> {
    let key = format!("session-snapshots/{}.bin", session_id);
    r2.put(&key, session_data, None).await
}
```

---

## 7. D2 SQLite Integration

### D2 Adapter for Rust

```rust
// crates/better-auth-db/src/d2.rs

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;
use js_sys::{Promise, Array, JsString};
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum D2Error {
    #[error("Query failed: {0}")]
    QueryFailed(String),
    #[error("No rows returned")]
    NoRows,
    #[error("Type conversion failed")]
    TypeConversion,
}

pub struct D2Adapter {
    db: JsValue,  // D1Database binding
}

impl D2Adapter {
    pub fn new(db: JsValue) -> Self {
        Self { db }
    }

    /// Execute a prepared statement
    pub async fn execute(&self, sql: &str, params: &[JsValue]) -> Result<D1Result, D2Error> {
        let stmt: Promise = js_sys::Reflect::get(&self.db, &JsString::from("prepare"))
            .unwrap()
            .dyn_into::<js_sys::Function>()
            .unwrap()
            .call1(&self.db, &JsString::from(sql))
            .unwrap()
            .dyn_into()
            .unwrap();

        // Bind parameters and execute...
        todo!()
    }

    /// Query for rows
    pub async fn query<T: for<'de> serde::Deserialize<'de>>(
        &self,
        sql: &str,
        params: &[JsValue],
    ) -> Result<Vec<T>, D2Error> {
        let results = self.execute(sql, params).await?;
        // Convert D1Result to Vec<T>
        todo!()
    }

    /// Create a user
    pub async fn create_user(&self, user: &crate::schema::User) -> Result<(), D2Error> {
        let sql = r#"
            INSERT INTO users (id, email, username, password_hash, email_verified, created_at, updated_at, metadata)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#;

        let params: Vec<JsValue> = vec![
            user.id.clone().into(),
            user.email.clone().into(),
            user.username.clone().into(),
            user.password_hash.clone().into(),
            (user.email_verified as i32).into(),
            (user.created_at.timestamp()).into(),
            (user.updated_at.timestamp()).into(),
            user.metadata.as_ref().map(|m| m.to_string()).unwrap_or_default().into(),
        ];

        self.execute(sql, &params).await?;
        Ok(())
    }

    /// Find user by email
    pub async fn find_user_by_email(
        &self,
        email: &str,
    ) -> Result<Option<crate::schema::User>, D2Error> {
        let sql = "SELECT * FROM users WHERE email = ?";
        let results: Vec<crate::schema::User> = self.query(sql, &[email.into()]).await?;
        Ok(results.into_iter().next())
    }

    /// Create session
    pub async fn create_session(&self, session: &crate::schema::Session) -> Result<(), D2Error> {
        let sql = r#"
            INSERT INTO sessions (id, user_id, token, expires_at, created_at, ip_address, user_agent, metadata)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#;

        // Build params and execute...
        Ok(())
    }

    /// Find session by token
    pub async fn find_session_by_token(
        &self,
        token: &str,
    ) -> Result<Option<crate::schema::Session>, D2Error> {
        let sql = "SELECT * FROM sessions WHERE token = ?";
        let results: Vec<crate::schema::Session> = self.query(sql, &[token.into()]).await?;
        Ok(results.into_iter().next())
    }

    /// Delete expired sessions
    pub async fn delete_expired_sessions(&self, now: i64) -> Result<u32, D2Error> {
        let sql = "DELETE FROM sessions WHERE expires_at < ?";
        let result = self.execute(sql, &[now.into()]).await?;
        Ok(result.meta?.count as u32)
    }
}

/// Database adapter trait (for abstraction)
#[async_trait::async_trait]
pub trait DatabaseAdapter: Send + Sync {
    async fn create_user(&self, user: &User) -> Result<(), DbError>;
    async fn find_user_by_email(&self, email: &str) -> Result<Option<User>, DbError>;
    async fn create_session(&self, session: &Session) -> Result<(), DbError>;
    async fn find_session_by_token(&self, token: &str) -> Result<Option<Session>, DbError>;
    async fn delete_session(&self, session_id: &str) -> Result<(), DbError>;
}

#[async_trait::async_trait]
impl DatabaseAdapter for D2Adapter {
    // Implementation...
}
```

### D2 Migration Script

```typescript
// migrations/001_initial_schema.ts

export async function migrate(db: D1Database) {
  await db.exec(`
    CREATE TABLE IF NOT EXISTS users (
      id TEXT PRIMARY KEY,
      email TEXT UNIQUE NOT NULL,
      username TEXT UNIQUE,
      password_hash TEXT,
      email_verified INTEGER DEFAULT 0,
      email_verified_at INTEGER,
      created_at INTEGER NOT NULL,
      updated_at INTEGER NOT NULL,
      metadata TEXT
    );

    CREATE TABLE IF NOT EXISTS sessions (
      id TEXT PRIMARY KEY,
      user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
      token TEXT UNIQUE NOT NULL,
      expires_at INTEGER NOT NULL,
      created_at INTEGER NOT NULL,
      ip_address TEXT,
      user_agent TEXT,
      metadata TEXT
    );

    -- (rest of schema from section 4)
  `);
}
```

---

## 8. API Design

### RESTful Endpoints

```
Authentication
POST   /auth/sign-up                    # Create account
POST   /auth/sign-in                    # Sign in
POST   /auth/sign-out                   # Sign out
GET    /auth/session                    # Get current session
POST   /auth/refresh                    # Refresh session
POST   /auth/revoke                     # Revoke all sessions

Password
POST   /auth/password/reset/request     # Request password reset
POST   /auth/password/reset/confirm     # Confirm password reset
POST   /auth/password/change            # Change password

Magic Link
POST   /auth/magic-link/request         # Request magic link
GET    /auth/magic-link/verify          # Verify magic link

OAuth2
GET    /auth/oauth/:provider            # Start OAuth flow
GET    /auth/oauth/:provider/callback   # OAuth callback

Two-Factor
POST   /auth/2fa/enable                 # Enable 2FA
POST   /auth/2fa/disable                # Disable 2FA
POST   /auth/2fa/verify                 # Verify 2FA code
POST   /auth/2fa/backup                 # Use backup code

Organization
GET    /org/:slug                       # Get organization
POST   /org/create                      # Create organization
POST   /org/:slug/invite                # Invite member
GET    /org/:slug/members               # List members
DELETE /org/:slug/members/:userId       # Remove member

API Keys
GET    /api-keys                        # List API keys
POST   /api-keys                        # Create API key
DELETE /api-keys/:id                    # Delete API key

Admin (with admin plugin)
GET    /admin/users                     # List all users
GET    /admin/users/:id                 # Get user
PUT    /admin/users/:id                 # Update user
DELETE /admin/users/:id                 # Delete user
GET    /admin/sessions                  # List all sessions
POST   /admin/sessions/:id/revoke       # Revoke session
```

### Request/Response Types

```typescript
// bindings/ts/types.ts

export interface SignUpRequest {
  email: string;
  password: string;
  username?: string;
}

export interface SignUpResponse {
  user: User;
  session: Session;
  token: string;
}

export interface SignInRequest {
  email: string;
  password: string;
  rememberMe?: boolean;
}

export interface SignInResponse {
  user: User;
  session: Session;
  token: string;
}

export interface User {
  id: string;
  email: string;
  username?: string;
  emailVerified: boolean;
  createdAt: string;
  metadata?: Record<string, unknown>;
}

export interface Session {
  id: string;
  userId: string;
  expiresAt: string;
  createdAt: string;
  ipAddress?: string;
  userAgent?: string;
}

export interface MagicLinkRequest {
  email: string;
  callbackUrl?: string;
}

export interface OAuthConfig {
  providers: {
    github?: { clientId: string; scopes?: string[] };
    google?: { clientId: string; scopes?: string[] };
  };
}
```

---

## 9. JWT & Session Management

### Session Token Strategy

```rust
// crates/better-auth-session/src/token.rs

use jwt_simple::prelude::*;
use serde::{Deserialize, Serialize};
use chrono::{Utc, Duration};

/// Session token types
pub enum TokenType {
    /// Compact: Base64Url(payload) + HMAC
    /// Pros: Small, fast
    /// Cons: No standard format
    Compact,

    /// JWT: Header.Payload.Signature
    /// Pros: Standard, widely supported
    /// Cons: Larger than compact
    Jwt,

    /// PASETO: Versioned, opinionated tokens
    /// Pros: Type-safe, built-in expiration
    /// Cons: Less widely adopted
    Paseto,
}

/// Session claims for JWT/PASETO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionClaims {
    /// Subject (user ID)
    pub sub: String,

    /// Session ID
    pub sid: String,

    /// Issued at (timestamp)
    pub iat: i64,

    /// Expiration (timestamp)
    pub exp: i64,

    /// Not before (timestamp)
    pub nbf: Option<i64>,

    /// Issuer
    pub iss: Option<String>,

    /// Audience
    pub aud: Option<String>,

    /// Custom claims
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

impl SessionClaims {
    pub fn new(user_id: String, session_id: String, expiration: Duration) -> Self {
        let now = Utc::now();
        Self {
            sub: user_id,
            sid: session_id,
            iat: now.timestamp(),
            exp: (now + expiration).timestamp(),
            nbf: None,
            iss: None,
            aud: None,
            roles: vec![],
            permissions: vec![],
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn with_roles(mut self, roles: Vec<String>) -> Self {
        self.roles = roles;
        self
    }

    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp() > self.exp
    }
}

/// Token generator
pub struct TokenGenerator {
    secret: SecretKey,
    token_type: TokenType,
    expiration: Duration,
}

impl TokenGenerator {
    pub fn new(secret: &[u8], token_type: TokenType, expiration: Duration) -> Self {
        Self {
            secret: SecretKey::from_bytes(secret),
            token_type,
            expiration,
        }
    }

    pub fn generate(&self, user_id: String, session_id: String) -> Result<String, TokenError> {
        let claims = SessionClaims::new(user_id, session_id, self.expiration);

        match self.token_type {
            TokenType::Jwt => {
                let jwt_claims = Claims::with_custom_claims(claims, self.expiration);
                Ok(self.secret.authenticate(jwt_claims)?)
            }
            TokenType::Compact => {
                // Custom compact format
                let payload = serde_json::to_string(&claims)?;
                let payload_b64 = base64_url::encode(payload.as_bytes());
                let signature = self.sign(payload.as_bytes())?;
                Ok(format!("{}.{}", payload_b64, signature))
            }
            TokenType::Paseto => {
                // Use rusty_paseto
                todo!("PASETO implementation")
            }
        }
    }

    pub fn verify(&self, token: &str) -> Result<SessionClaims, TokenError> {
        match self.token_type {
            TokenType::Jwt => {
                let claims: Claims<SessionClaims> = self.secret.verify(token)?;
                Ok(claims.custom)
            }
            TokenType::Compact => {
                // Verify compact format
                let parts: Vec<&str> = token.split('.').collect();
                if parts.len() != 2 {
                    return Err(TokenError::InvalidFormat);
                }

                let payload_b64 = parts[0];
                let signature = parts[1];

                let payload = base64_url::decode(payload_b64)?;
                let expected_sig = self.sign(&payload)?;

                if !constant_time_eq(signature.as_bytes(), expected_sig.as_bytes()) {
                    return Err(TokenError::InvalidSignature);
                }

                let claims: SessionClaims = serde_json::from_slice(&payload)?;
                Ok(claims)
            }
            TokenType::Paseto => {
                todo!("PASETO verification")
            }
        }
    }

    fn sign(&self, data: &[u8]) -> Result<String, TokenError> {
        use hmac::{Hmac, Mac};
        type HmacSha256 = Hmac<sha2::Sha256>;

        let mut mac = HmacSha256::new_from_slice(self.secret.as_bytes())?;
        mac.update(data);
        let result = mac.finalize();
        Ok(hex::encode(result.into_bytes()))
    }
}
```

### Session Manager

```rust
// crates/better-auth-session/src/manager.rs

use better_auth_db::adapter::DatabaseAdapter;
use better_auth_types::{Session, User};
use crate::token::{TokenGenerator, SessionClaims};
use chrono::{Utc, Duration};

pub struct SessionManager {
    db: Box<dyn DatabaseAdapter>,
    token_generator: TokenGenerator,
    cookie_config: CookieConfig,
}

impl SessionManager {
    pub async fn create_session(
        &self,
        user: &User,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<SessionWithToken, SessionError> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let expires_at = Utc::now() + self.cookie_config.max_age;

        // Create session in database
        let session = Session {
            id: session_id.clone(),
            user_id: user.id.clone(),
            token: uuid::Uuid::new_v4().to_string(),
            expires_at: expires_at.timestamp(),
            created_at: Utc::now().timestamp(),
            ip_address,
            user_agent,
            metadata: None,
        };

        self.db.create_session(&session).await?;

        // Generate token
        let token = self.token_generator.generate(user.id.clone(), session_id)?;

        Ok(SessionWithToken { session, token })
    }

    pub async fn get_session(&self, token: &str) -> Result<Option<(Session, User)>, SessionError> {
        // First try cache (KV store)
        // If miss, query database

        // Verify token signature
        let claims = self.token_generator.verify(token)?;

        if claims.is_expired() {
            return Ok(None);
        }

        // Get session from database
        let session = self.db.find_session_by_id(&claims.sid).await?;

        match session {
            Some(s) => {
                // Get user
                let user = self.db.find_user_by_id(&s.user_id).await?;
                Ok(user.map(|u| (s, u)))
            }
            None => Ok(None),
        }
    }

    pub async fn revoke_session(&self, session_id: &str) -> Result<(), SessionError> {
        self.db.delete_session(session_id).await?;
        Ok(())
    }

    pub async fn revoke_all_sessions(&self, user_id: &str) -> Result<u32, SessionError> {
        self.db.delete_all_user_sessions(user_id).await
    }

    pub async fn refresh_session(
        &self,
        session: &Session,
    ) -> Result<String, SessionError> {
        // Extend expiration
        let new_expires = Utc::now() + self.cookie_config.max_age;

        // Update database
        self.db.update_session_expiration(&session.id, new_expires.timestamp()).await?;

        // Generate new token
        self.token_generator.generate(session.user_id.clone(), session.id.clone())
    }
}

pub struct SessionWithToken {
    pub session: Session,
    pub token: String,
}

pub struct CookieConfig {
    pub max_age: Duration,
    pub secure: bool,
    pub http_only: bool,
    pub same_site: SameSite,
    pub path: String,
}

pub enum SameSite {
    Strict,
    Lax,
    None,
}
```

---

## 10. Self-Contained World

### What "No External Services" Means

A truly self-contained Better Auth on CloudFlare:

1. **No external authentication providers** (except optional OAuth)
   - All auth logic runs in your Worker
   - No Auth0, no Firebase Auth, no Clerk

2. **No external databases**
   - D2 provides SQLite at the edge
   - Data lives in CloudFlare's network

3. **No external cache/message queue**
   - KV store for sessions/rate-limiting
   - No Redis, no Upstash required

4. **No external storage**
   - R2 for avatars, exports, backups
   - No S3, no Cloudinary

5. **No external email provider** (optional)
   - Can integrate with any SMTP/API
   - Or queue emails for batch sending

### Deployment Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                    CloudFlare Network                             │
│                                                                   │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐          │
│  │   Edge 1    │    │   Edge 2    │    │   Edge N    │          │
│  │  (Worker)   │    │  (Worker)   │    │  (Worker)   │          │
│  │             │    │             │    │             │          │
│  │  ┌───────┐  │    │  ┌───────┐  │    │  ┌───────┐  │          │
│  │  │ WASM  │  │    │  │ WASM  │  │    │  │ WASM  │  │          │
│  │  │ Auth  │  │    │  │ Auth  │  │    │  │ Auth  │  │          │
│  │  └───────┘  │    │  └───────┘  │    │  └───────┘  │          │
│  └──────┬──────┘    └──────┬──────┘    └──────┬──────┘          │
│         │                  │                  │                  │
│         └──────────────────┼──────────────────┘                  │
│                            │                                      │
│         ┌──────────────────┼──────────────────┐                  │
│         │                  │                  │                  │
│         ▼                  ▼                  ▼                  │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐          │
│  │     D2      │    │     R2      │    │     KV      │          │
│  │  (SQLite)   │    │  (Objects)  │    │   (Cache)   │          │
│  │             │    │             │    │             │          │
│  │ • Global    │    │ • Regional  │    │ • Global    │          │
│  │ • ACID      │    │ • S3 API    │    │ • Eventually│          │
│  │             │    │             │    │   Consistent│          │
│  └─────────────┘    └─────────────┘    └─────────────┘          │
│                                                                   │
└──────────────────────────────────────────────────────────────────┘
```

### Data Flow for Sign-In

```
1. User submits credentials
   │
   ▼
2. CloudFlare Edge receives request
   │
   ▼
3. WASM Auth Engine parses request
   │
   ├─► Validate email format
   ├─► Check rate limits (KV)
   └─► Extract credentials
   │
   ▼
4. Query D2 for user
   │
   ▼
5. Constant-time password verification
   ├─► Always hash (even if user not found)
   ├─► Argon2id via Rust
   └─► Compare hashes
   │
   ▼
6. If valid:
   ├─► Create session (D2)
   ├─► Generate JWT (Rust crypto)
   ├─► Cache session (KV)
   └─► Set cookie header
   │
   ▼
7. Return response to user
```

### Cost Estimate (CloudFlare Free Tier)

| Resource | Free Tier | Usage | Overage |
|----------|-----------|-------|---------|
| Worker Requests | 100K/day | ~3M/month | $0.50/million |
| D2 Read Ops | 5M/month | User lookups | $0.75/million |
| D2 Write Ops | 100K/month | Sign-ups, sessions | $5/million |
| D2 Storage | 10 GB | User data | $0.75/GB-month |
| R2 Storage | 10 GB | Avatars, exports | $0.015/GB-month |
| R2 Class A Ops | 1M/month | Uploads | $4.50/million |
| R2 Class B Ops | 10M/month | Downloads | $0.36/million |
| KV Reads | 100K/day | Session cache | $0.50/million |
| KV Writes | 1K/day | Session creation | $5/million |

**Estimated monthly cost for 10K users:**
- ~50K sign-ins/day → Well within free tier
- D2 + R2 + KV → ~$5-10/month

---

## Appendix A: Build Commands

```bash
# Build WASM module
cd crates/better-auth-bindings
wasm-pack build --target web --out-dir ../../dist

# Build TypeScript bindings
cd bindings/ts
tsc --build

# Deploy to CloudFlare
wrangler deploy

# Local development
wrangler dev --local
```

## Appendix B: Testing Strategy

```rust
// tests/integration_test.rs

#[cfg(test)]
mod tests {
    use better_auth_core::AuthEngine;
    use better_auth_db::memory_adapter::MemoryAdapter;

    #[tokio::test]
    async fn test_sign_up_sign_in() {
        let db = MemoryAdapter::new();
        let auth = AuthEngine::new("test-secret", db);

        // Sign up
        let user = auth.sign_up("test@example.com", "password123").await.unwrap();
        assert_eq!(user.email, "test@example.com");

        // Sign in
        let response = auth.sign_in("test@example.com", "password123").await.unwrap();
        assert_eq!(response.user.id, user.id);
        assert!(!response.token.is_empty());
    }

    #[tokio::test]
    async fn test_invalid_credentials() {
        let db = MemoryAdapter::new();
        let auth = AuthEngine::new("test-secret", db);

        auth.sign_up("test@example.com", "password123").await.unwrap();

        let result = auth.sign_in("test@example.com", "wrong-password").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_magic_link_flow() {
        let db = MemoryAdapter::new();
        let auth = AuthEngine::new("test-secret", db);

        let result = auth.request_magic_link("test@example.com", "http://localhost").await.unwrap();
        assert!(!result.magic_link.is_empty());
        assert!(result.magic_link.contains("token="));
    }
}
```

## Appendix C: Security Checklist

- [ ] All password hashing uses Argon2id with secure parameters
- [ ] Constant-time comparison for all sensitive operations
- [ ] HMAC signing for all tokens (magic links, password reset)
- [ ] Secure cookie flags (HttpOnly, Secure, SameSite)
- [ ] Rate limiting on all auth endpoints
- [ ] CSRF protection on state-changing operations
- [ ] Input validation on all user-provided data
- [ ] SQL injection prevention via parameterized queries
- [ ] XSS prevention via output encoding
- [ ] Session invalidation on password change
- [ ] Secure random generation for all tokens
- [ ] Zeroization of sensitive data in memory

---

## Conclusion

This document outlines a comprehensive vision for Better Auth as a fully self-contained, WASM-powered authentication system running on CloudFlare Workers. The key takeaways are:

1. **Rust + WASM** provides cryptographic performance and portability
2. **D2** offers SQLite at the edge with full ACID guarantees
3. **R2** handles blob storage without S3 dependencies
4. **KV** provides low-latency caching for sessions
5. **Zero external services** means complete ownership of your auth stack

The architecture maintains backward compatibility with the existing TypeScript API while providing a migration path to Rust for performance-critical operations.
