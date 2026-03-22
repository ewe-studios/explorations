---
name: JWKS
description: JSON Web Key Set implementation for SpacetimeDB authentication and key management
type: sub-project
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.SpacetimeDB/jwks/
---

# JWKS - JSON Web Key Set for SpacetimeDB

## Overview

JWKS (JSON Web Key Set) is a **cryptographic key management implementation** for SpacetimeDB that provides secure key distribution and validation for JWT authentication. It enables SpacetimeDB applications to verify JWT tokens issued by external identity providers (Auth0, Okta, AWS Cognito, etc.) using public keys from a JWKS endpoint.

Key features:
- **JWKS fetching** - Automatic key set retrieval
- **Key caching** - Efficient key management with TTL
- **Multiple algorithms** - RS256, ES256, EdDSA support
- **Key rotation** - Automatic handling of rotated keys
- **SpacetimeDB integration** - Built for SpacetimeDB auth

## Directory Structure

```
jwks/
├── src/
│   ├── lib.rs              # Main module
│   ├── client.rs           # JWKS HTTP client
│   ├── cache.rs            # Key caching layer
│   ├── key.rs              # Key representation
│   ├── validator.rs        # JWT validation
│   └── error.rs            # Error types
├── Cargo.toml
└── README.md
```

## Core Components

### JWKS Client

```rust
use reqwest::Client;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Jwks {
    pub keys: Vec<Jwk>,
}

#[derive(Debug, Deserialize)]
pub struct Jwk {
    pub kty: String,
    pub kid: Option<String>,
    pub use_: Option<String>,
    pub alg: Option<String>,

    // RSA-specific
    pub n: Option<String>,
    pub e: Option<String>,

    // EC-specific
    pub crv: Option<String>,
    pub x: Option<String>,
    pub y: Option<String>,
}

pub struct JwksClient {
    http: Client,
    issuer_url: String,
    cache: JwksCache,
}

impl JwksClient {
    pub fn new(issuer_url: String) -> Self {
        Self {
            http: Client::new(),
            issuer_url,
            cache: JwksCache::new(Duration::from_secs(3600)),
        }
    }

    pub async fn get_keys(&self) -> Result<&Jwks, JwksError> {
        // Check cache first
        if let Some(keys) = self.cache.get() {
            return Ok(keys);
        }

        // Fetch from JWKS endpoint
        let jwks_url = format!("{}/.well-known/jwks.json", self.issuer_url);
        let response = self.http.get(&jwks_url).send().await?;
        let jwks: Jwks = response.json().await?;

        // Cache the result
        self.cache.set(jwks);

        Ok(self.cache.get().unwrap())
    }

    pub async fn get_key(&self, kid: &str) -> Result<&Jwk, JwksError> {
        let jwks = self.get_keys().await?;

        jwks.keys
            .iter()
            .find(|k| k.kid.as_deref() == Some(kid))
            .ok_or(JwksError::KeyNotFound(kid.to_string()))
    }
}
```

### Key Cache

```rust
use std::sync::RwLock;
use std::time::{Duration, Instant};

pub struct JwksCache {
    inner: RwLock<Option<CachedJwks>>,
    ttl: Duration,
}

struct CachedJwks {
    jwks: Jwks,
    fetched_at: Instant,
}

impl JwksCache {
    pub fn new(ttl: Duration) -> Self {
        Self {
            inner: RwLock::new(None),
            ttl,
        }
    }

    pub fn get(&self) -> Option<&Jwks> {
        let guard = self.inner.read().ok()?;
        let cached = guard.as_ref()?;

        // Check if expired
        if cached.fetched_at.elapsed() > self.ttl {
            return None;
        }

        Some(&cached.jwks)
    }

    pub fn set(&self, jwks: Jwks) {
        let cached = CachedJwks {
            jwks,
            fetched_at: Instant::now(),
        };

        if let Ok(mut guard) = self.inner.write() {
            *guard = Some(cached);
        }
    }

    pub fn invalidate(&self) {
        if let Ok(mut guard) = self.inner.write() {
            *guard = None;
        }
    }
}
```

### JWT Validator

```rust
use jsonwebtoken::{decode, Validation, Algorithm, DecodingKey};

pub struct JwtValidator {
    jwks_client: JwksClient,
    issuer: String,
    audience: String,
}

impl JwtValidator {
    pub fn new(issuer: String, audience: String) -> Self {
        Self {
            jwks_client: JwksClient::new(issuer.clone()),
            issuer,
            audience,
        }
    }

    pub async fn validate(&self, token: &str) -> Result<Claims, JwtError> {
        // Parse header to get kid
        let header = jsonwebtoken::decode_header(token)?;

        // Validate algorithm
        let alg = header.alg.parse::<Algorithm>()
            .map_err(|_| JwtError::UnsupportedAlgorithm(header.alg))?;

        // Get key from JWKS
        let kid = header.kid
            .ok_or(JwtError::MissingKeyId)?;

        let jwk = self.jwks_client.get_key(&kid).await?;

        // Convert JWK to DecodingKey
        let decoding_key = jwk_to_decoding_key(jwk)?;

        // Set up validation
        let mut validation = Validation::new(alg);
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&[&self.audience]);

        // Decode and validate token
        let token_data = decode::<Claims>(
            token,
            &decoding_key,
            &validation
        )?;

        Ok(token_data.claims)
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct Claims {
    pub sub: String,
    pub iss: String,
    pub aud: String,
    pub exp: u64,
    pub iat: u64,
    pub email: Option<String>,
    pub name: Option<String>,
}
```

### JWK to Decoding Key Conversion

```rust
use jsonwebtoken::DecodingKey;
use rsa::{RsaPublicKey, BigUint};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};

fn jwk_to_decoding_key(jwk: &Jwk) -> Result<DecodingKey, JwksError> {
    match jwk.kty.as_str() {
        "RSA" => {
            let n = jwk.n.as_ref().ok_or(JwksError::MissingRsaN)?;
            let e = jwk.e.as_ref().ok_or(JwksError::MissingRsaE)?;

            let n_bytes = URL_SAFE_NO_PAD.decode(n)?;
            let e_bytes = URL_SAFE_NO_PAD.decode(e)?;

            let n = BigUint::from_bytes_be(&n_bytes);
            let e = BigUint::from_bytes_be(&e_bytes);

            let public_key = RsaPublicKey::new(n, e)?;
            let pem = public_key.to_public_key_pem()?;

            Ok(DecodingKey::from_rsa_pem(&pem)?)
        }

        "EC" => {
            let x = jwk.x.as_ref().ok_or(JwksError::MissingEcX)?;
            let y = jwk.y.as_ref().ok_or(JwksError::MissingEcY)?;
            let crv = jwk.crv.as_ref().ok_or(JwksError::MissingEcCrv)?;

            // Convert EC JWK to PEM
            let pem = ec_jwk_to_pem(x, y, crv)?;
            Ok(DecodingKey::from_ec_pem(&pem)?)
        }

        "OKP" => {
            // EdDSA keys
            let x = jwk.x.as_ref().ok_or(JwksError::MissingEdX)?;
            let x_bytes = URL_SAFE_NO_PAD.decode(x)?;

            Ok(DecodingKey::from_ed_der(&x_bytes))
        }

        _ => Err(JwksError::UnsupportedKeyType(jwk.kty.clone())),
    }
}
```

## SpacetimeDB Integration

### Authentication Reducer

```rust
use spacetimedb::{ReducerContext, Identity};
use jwks::JwtValidator;

#[spacetimedb(table)]
pub struct AuthenticatedUser {
    #[primarykey]
    pub id: Identity,
    pub jwt_sub: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub authenticated_at: Timestamp,
}

#[spacetimedb(reducer)]
pub fn authenticate(
    ctx: ReducerContext,
    token: String,
) -> Result<(), AuthError> {
    // Initialize validator (should be singleton in production)
    let validator = get_validator();

    // Validate JWT
    let claims = validator.validate(&token)
        .await
        .map_err(|e| AuthError::InvalidToken(e.to_string()))?;

    // Check expiration
    if claims.exp < current_timestamp() {
        return Err(AuthError::TokenExpired);
    }

    // Store authenticated user
    ctx.authenticated_user.insert(AuthenticatedUser {
        id: ctx.sender,
        jwt_sub: claims.sub,
        email: claims.email,
        name: claims.name,
        authenticated_at: Timestamp::now(),
    });

    Ok(())
}

#[spacetimedb(reducer)]
pub fn logout(ctx: ReducerContext) -> Result<(), AuthError> {
    ctx.authenticated_user.delete(ctx.sender);
    Ok(())
}
```

### Auth Guard Pattern

```rust
use spacetimedb::{ReducerContext, Identity};

fn require_auth(ctx: &ReducerContext) -> Result<AuthenticatedUser, AuthError> {
    ctx.authenticated_user
        .find(|u| u.id == ctx.sender)
        .ok_or(AuthError::NotAuthenticated)
}

#[spacetimedb(reducer)]
pub fn create_post(
    ctx: ReducerContext,
    title: String,
    content: String,
) -> Result<(), AuthError> {
    // Require authentication
    let user = require_auth(&ctx)?;

    ctx.post.insert(Post {
        id: generate_id(),
        author_id: user.id,
        title,
        content,
        created_at: Timestamp::now(),
    });

    Ok(())
}
```

### Middleware Pattern

```rust
// Client-side auth middleware
use jwks::JwtValidator;

pub struct AuthMiddleware {
    validator: JwtValidator,
    token_store: TokenStore,
}

impl AuthMiddleware {
    pub fn new(issuer: String, audience: String) -> Self {
        Self {
            validator: JwtValidator::new(issuer, audience),
            token_store: TokenStore::new(),
        }
    }

    pub async fn authenticate(&self) -> Result<(), AuthError> {
        let token = self.token_store.get()
            .ok_or(AuthError::NoToken)?;

        let claims = self.validator.validate(&token).await?;

        // Call SpacetimeDB auth reducer
        db.auth.authenticate(token).await?;

        Ok(())
    }

    pub fn is_authenticated(&self) -> bool {
        self.token_store.has_valid_token()
    }

    pub fn logout(&self) {
        self.token_store.clear();
        db.auth.logout().await.ok();
    }
}
```

## Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum JwksError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Key not found: {0}")]
    KeyNotFound(String),

    #[error("Missing RSA n parameter")]
    MissingRsaN,

    #[error("Missing RSA e parameter")]
    MissingRsaE,

    #[error("Missing EC x parameter")]
    MissingEcX,

    #[error("Missing EC y parameter")]
    MissingEcY,

    #[error("Missing EC crv parameter")]
    MissingEcCrv,

    #[error("Missing EdDSA x parameter")]
    MissingEdX,

    #[error("Unsupported key type: {0}")]
    UnsupportedKeyType(String),

    #[error("Unsupported algorithm: {0}")]
    UnsupportedAlgorithm(String),

    #[error("Base64 decode error: {0}")]
    Base64(#[from] base64::DecodeError),

    #[error("RSA error: {0}")]
    Rsa(#[from] rsa::Error),

    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum JwtError {
    #[error("Token expired")]
    TokenExpired,

    #[error("Missing key ID (kid) in token header")]
    MissingKeyId,

    #[error("Invalid token: {0}")]
    InvalidToken(String),

    #[error("JWKS error: {0}")]
    Jwks(#[from] JwksError),
}
```

## Related Documents

- [SpacetimeDB Cookbook](./spacetimedb-cookbook-exploration.md) - Auth patterns
- [Blackholio](./blackholio-exploration.md) - Multiplayer auth example

## Sources

- Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.SpacetimeDB/jwks/`
- JWKS Spec: https://datatracker.ietf.org/doc/html/rfc7517
- SpacetimeDB Auth: https://spacetimedb.com/docs/auth
