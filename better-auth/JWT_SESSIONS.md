---
type: reference
created: 2026-03-17
---

# JWT & Session Management Reference

Deep dive into JWT token formats, session management strategies, and cookie handling for Better Auth WASM.

## Table of Contents

1. [Token Formats](#token-formats)
2. [JWT Implementation](#jwt-implementation)
3. [PASETO Implementation](#paseto-implementation)
4. [Cookie Strategies](#cookie-strategies)
5. [Session Management](#session-management)
6. [Security Considerations](#security-considerations)

---

## Token Formats

### Comparison

| Format | Size | Speed | Standard | Features |
|--------|------|-------|----------|----------|
| Compact | ~200b | Fastest | Custom | Minimal |
| JWT | ~400b | Fast | RFC 7519 | Claims, signatures |
| PASETO | ~400b | Fast | PASETO spec | Versioned, opinionated |

### Token Structure Comparison

```
Compact Token:
┌─────────────────────────────────┬──────────────────────────────┐
│      Base64URL(JSON Payload)    │       HMAC-SHA256 Sig        │
│     {"sub":"user","exp":123}    │     64 hex characters        │
└─────────────────────────────────┴──────────────────────────────┘

JWT Token:
┌─────────────┬─────────────────────────────┬──────────────────────┐
│   Header    │          Payload            │      Signature       │
│ {"alg":"HS256│ {"sub":"user","exp":123,   │   HMAC-SHA256 Sig    │
│  ,"typ":"JWT"}│  "iat":100,"sid":"xyz"}   │                      │
└─────────────┴─────────────────────────────┴──────────────────────┘

PASETO V4 Local:
┌────────┬──────────────┬─────────────────────────────────────────┐
│  V4    │     Footer   │           Encrypted + Auth              │
│ (1 byte)│  (optional)  │   XChaCha20-Poly1305 ciphertext         │
└────────┴──────────────┴─────────────────────────────────────────┘
```

---

## JWT Implementation

### Rust JWT Module

```rust
// crates/better-auth-crypto/src/jwt.rs

use jwt_simple::prelude::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use chrono::{Utc, Duration};

#[derive(Error, Debug)]
pub enum JwtError {
    #[error("Invalid token format")]
    InvalidFormat,
    #[error("Token expired")]
    Expired,
    #[error("Invalid issuer")]
    InvalidIssuer,
    #[error("Invalid audience")]
    InvalidAudience,
    #[error("Invalid claims")]
    InvalidClaims,
    #[error("Signing failed")]
    SigningFailed,
    #[error("Verification failed")]
    VerificationFailed,
}

/// Session claims for JWT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionClaims {
    /// Subject - User ID
    pub sub: String,

    /// Session ID
    pub sid: String,

    /// Issued at (unix timestamp)
    pub iat: i64,

    /// Expiration (unix timestamp)
    pub exp: i64,

    /// Not before (unix timestamp)
    pub nbf: Option<i64>,

    /// Issuer
    pub iss: Option<String>,

    /// Audience
    pub aud: Option<String>,

    /// JWT ID (unique token identifier)
    pub jti: Option<String>,

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
            jti: Some(uuid::Uuid::new_v4().to_string()),
            roles: vec![],
            permissions: vec![],
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn with_issuer(mut self, issuer: String) -> Self {
        self.iss = Some(issuer);
        self
    }

    pub fn with_audience(mut self, audience: String) -> Self {
        self.aud = Some(audience);
        self
    }

    pub fn with_roles(mut self, roles: Vec<String>) -> Self {
        self.roles = roles;
        self
    }

    pub fn with_permissions(mut self, permissions: Vec<String>) -> Self {
        self.permissions = permissions;
        self
    }

    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp() >= self.exp
    }

    pub fn is_not_yet_valid(&self) -> bool {
        if let Some(nbf) = self.nbf {
            Utc::now().timestamp() < nbf
        } else {
            false
        }
    }
}

/// JWT Configuration
pub struct JwtConfig {
    pub secret: SecretKey,
    pub expiration: Duration,
    pub issuer: Option<String>,
    pub audience: Option<String>,
}

impl JwtConfig {
    pub fn new(secret: &[u8], expiration_hours: u64) -> Self {
        Self {
            secret: SecretKey::from_bytes(secret),
            expiration: Duration::hours(expiration_hours as i64),
            issuer: None,
            audience: None,
        }
    }

    pub fn with_issuer(mut self, issuer: String) -> Self {
        self.issuer = Some(issuer);
        self
    }

    pub fn with_audience(mut self, audience: String) -> Self {
        self.audience = Some(audience);
        self
    }
}

/// JWT Signer/Verifier
pub struct JwtSigner {
    config: JwtConfig,
}

impl JwtSigner {
    pub fn new(config: JwtConfig) -> Self {
        Self { config }
    }

    /// Sign claims and return JWT token
    pub fn sign(&self, claims: SessionClaims) -> Result<String, JwtError> {
        // Add standard claims if not set
        let mut claims = claims;
        if claims.iss.is_none() {
            claims.iss = self.config.issuer.clone();
        }
        if claims.aud.is_none() {
            claims.aud = self.config.audience.clone();
        }

        // Create JWT claims with expiration
        let jwt_claims = Claims::with_custom_claims(claims, self.config.expiration);

        // Sign and serialize
        self.config
            .secret
            .authenticate(jwt_claims)
            .map_err(|_| JwtError::SigningFailed)
    }

    /// Verify JWT token and return claims
    pub fn verify(&self, token: &str) -> Result<SessionClaims, JwtError> {
        // Create verification options
        let mut verification = VerificationOptions {
            accept_expired: false,
            ..Default::default()
        };

        if let Some(ref issuer) = self.config.issuer {
            verification.allowed_issuers = Some(HashSet::from_strings(&[issuer]));
        }

        if let Some(ref audience) = self.config.audience {
            verification.allowed_audiences = Some(HashSet::from_strings(&[audience]));
        }

        // Verify and decode
        let decoded = self
            .config
            .secret
            .verify::<SessionClaims>(token)
            .map_err(|_| JwtError::VerificationFailed)?;

        // Check expiration manually for better error
        if decoded.custom.is_expired() {
            return Err(JwtError::Expired);
        }

        Ok(decoded.custom)
    }

    /// Verify token without checking expiration (for refresh)
    pub fn verify_ignore_expiry(&self, token: &str) -> Result<SessionClaims, JwtError> {
        let mut verification = VerificationOptions {
            accept_expired: true,
            ..Default::default()
        };

        if let Some(ref issuer) = self.config.issuer {
            verification.allowed_issuers = Some(HashSet::from_strings(&[issuer]));
        }

        let decoded = self
            .config
            .secret
            .verify::<SessionClaims>(token)
            .map_err(|_| JwtError::VerificationFailed)?;

        Ok(decoded.custom)
    }

    /// Refresh a token (create new token from old claims)
    pub fn refresh(&self, old_token: &str) -> Result<String, JwtError> {
        let claims = self.verify_ignore_expiry(old_token)?;

        // Create new claims with fresh expiration
        let new_claims = SessionClaims::new(
            claims.sub,
            claims.sid,
            self.config.expiration,
        )
        .with_issuer(claims.iss.unwrap_or_default())
        .with_roles(claims.roles)
        .with_permissions(claims.permissions);

        self.sign(new_claims)
    }
}

/// Utility for creating HashSet from string slices
fn string_slice_to_set(strings: &[&str]) -> std::collections::HashSet<String> {
    strings.iter().map(|s| s.to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_and_verify() {
        let config = JwtConfig::new(b"test-secret-min-32-characters!!!", 24);
        let signer = JwtSigner::new(config);

        let claims = SessionClaims::new(
            "user-123".to_string(),
            "session-456".to_string(),
            Duration::hours(24),
        );

        let token = signer.sign(claims.clone()).unwrap();
        let verified = signer.verify(&token).unwrap();

        assert_eq!(verified.sub, claims.sub);
        assert_eq!(verified.sid, claims.sid);
    }

    #[test]
    fn test_expired_token() {
        let config = JwtConfig::new(b"test-secret-min-32-characters!!!", 0);
        let signer = JwtSigner::new(config);

        let claims = SessionClaims::new(
            "user-123".to_string(),
            "session-456".to_string(),
            Duration::seconds(-1),  // Already expired
        );

        let token = signer.sign(claims).unwrap();
        let result = signer.verify(&token);

        assert!(matches!(result, Err(JwtError::Expired)));
    }
}
```

---

## PASETO Implementation

```rust
// crates/better-auth-crypto/src/paseto.rs

use rusty_paseto::prelude::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use chrono::{Utc, Duration};

#[derive(Error, Debug)]
pub enum PasetoError {
    #[error("Invalid token format")]
    InvalidFormat,
    #[error("Token expired")]
    Expired,
    #[error("Decryption failed")]
    DecryptionFailed,
    #[error("Encryption failed")]
    EncryptionFailed,
    #[error("Invalid key")]
    InvalidKey,
}

/// PASETO V4 Local token (symmetric encryption)
pub struct PasetoLocal {
    key: Key32,
    expiration: Duration,
}

impl PasetoLocal {
    pub fn new(key: &[u8], expiration: Duration) -> Result<Self, PasetoError> {
        let key_bytes: [u8; 32] = key.try_into()
            .map_err(|_| PasetoError::InvalidKey)?;

        Ok(Self {
            key: Key32::from(key_bytes),
            expiration,
        })
    }

    /// Encrypt a payload into a PASETO token
    pub fn encrypt<T: Serialize>(&self, payload: &T) -> Result<String, PasetoError> {
        let mut builder = PasetoBuilder::new()
            .set_encryption_key(&self.key)
            .set_expiration(&(Utc::now() + self.expiration));

        // Serialize payload to JSON
        let json = serde_json::to_string(payload)
            .map_err(|_| PasetoError::EncryptionFailed)?;

        let token = builder
            .set_claim("data", json)
            .build()
            .map_err(|_| PasetoError::EncryptionFailed)?;

        Ok(token)
    }

    /// Decrypt a PASETO token
    pub fn decrypt<T: for<'de> Deserialize<'de>>(&self, token: &str) -> Result<T, PasetoError> {
        let parsed = Paseto::<Local, V4>::parse(token)
            .map_err(|_| PasetoError::InvalidFormat)?;

        // Verify expiration
        if let Some(exp) = parsed.claims().expiration() {
            if Utc::now().timestamp() >= exp.timestamp() {
                return Err(PasetoError::Expired);
            }
        }

        // Get the data claim
        let data = parsed
            .claims()
            .get_claim("data")
            .ok_or(PasetoError::InvalidFormat)?;

        let json = data.as_str().ok_or(PasetoError::InvalidFormat)?;

        serde_json::from_str(json)
            .map_err(|_| PasetoError::DecryptionFailed)
    }
}

/// PASETO V4 Public token (asymmetric - Ed25519)
pub struct PasetoPublic {
    private_key: crate::Ed25519KeyPair,
    public_key: crate::Ed25519PublicKey,
}

impl PasetoPublic {
    pub fn new(private_key: &[u8], public_key: &[u8]) -> Result<Self, PasetoError> {
        // Initialize key pairs (simplified)
        todo!()
    }

    pub fn sign<T: Serialize>(&self, payload: &T) -> Result<String, PasetoError> {
        todo!()
    }

    pub fn verify<T: for<'de> Deserialize<'de>>(&self, token: &str) -> Result<T, PasetoError> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct TestData {
        user_id: String,
        session_id: String,
    }

    #[test]
    fn test_encrypt_decrypt() {
        let key = b"super-secret-key-that-is-32-bytes!";
        let paseto = PasetoLocal::new(key, Duration::hours(24)).unwrap();

        let data = TestData {
            user_id: "user-123".to_string(),
            session_id: "session-456".to_string(),
        };

        let token = paseto.encrypt(&data).unwrap();
        let decrypted: TestData = paseto.decrypt(&token).unwrap();

        assert_eq!(data, decrypted);
    }
}
```

---

## Cookie Strategies

### Three-Cookie System

```rust
// crates/better-auth-session/src/cookie.rs

use chrono::{Duration, Utc};

/// Cookie configuration
pub struct CookieConfig {
    /// Cookie name prefix
    pub name_prefix: String,

    /// Session token cookie name
    pub session_token_name: String,

    /// Session data cookie name
    pub session_data_name: String,

    /// "Don't remember me" flag cookie name
    pub dont_remember_name: String,

    /// Cookie path
    pub path: String,

    /// Cookie domain (optional)
    pub domain: Option<String>,

    /// Secure flag (HTTPS only)
    pub secure: bool,

    /// HttpOnly flag (no JavaScript access)
    pub http_only: bool,

    /// SameSite policy
    pub same_site: SameSite,

    /// Session token max age
    pub token_max_age: Duration,

    /// Session data cache max age (shorter, reduces DB lookups)
    pub data_max_age: Duration,
}

impl Default for CookieConfig {
    fn default() -> Self {
        Self {
            name_prefix: "better-auth".to_string(),
            session_token_name: "better-auth.session-token".to_string(),
            session_data_name: "better-auth.session-data".to_string(),
            dont_remember_name: "better-auth.dont-remember".to_string(),
            path: "/".to_string(),
            domain: None,
            secure: true,
            http_only: true,
            same_site: SameSite::Lax,
            token_max_age: Duration::days(7),
            data_max_age: Duration::minutes(5),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum SameSite {
    Strict,
    Lax,
    None,
}

impl SameSite {
    pub fn as_str(&self) -> &'static str {
        match self {
            SameSite::Strict => "Strict",
            SameSite::Lax => "Lax",
            SameSite::None => "None",
        }
    }
}

/// Session cookie strategies
pub enum SessionStrategy {
    /// Compact: Base64 + HMAC
    /// Smallest size, custom format
    Compact,

    /// JWT: Standard JWT format
    /// Widely supported, larger than compact
    Jwt,

    /// JWE: Encrypted JWT
    /// Most secure, largest size
    Jwe,
}

/// Build Set-Cookie header
pub fn build_set_cookie_header(
    name: &str,
    value: &str,
    config: &CookieConfig,
    max_age: Option<Duration>,
) -> String {
    let mut parts = vec![format!("{}={}", name, value)];

    if let Some(ma) = max_age {
        parts.push(format!("Max-Age={}", ma.num_seconds()));
    }

    parts.push(format!("Path={}", config.path));

    if let Some(ref domain) = config.domain {
        parts.push(format!("Domain={}", domain));
    }

    if config.secure {
        parts.push("Secure".to_string());
    }

    if config.http_only {
        parts.push("HttpOnly".to_string());
    }

    parts.push(format!("SameSite={}", config.same_site.as_str()));

    parts.push("HttpOnly".to_string());

    parts.join("; ")
}

/// Build session cookies
pub struct SessionCookieBuilder {
    config: CookieConfig,
}

impl SessionCookieBuilder {
    pub fn new(config: CookieConfig) -> Self {
        Self { config }
    }

    /// Build session token cookie
    pub fn session_token(&self, token: &str) -> String {
        build_set_cookie_header(
            &self.config.session_token_name,
            token,
            &self.config,
            Some(self.config.token_max_age),
        )
    }

    /// Build session data cookie (encrypted)
    pub fn session_data(&self, encrypted_data: &str) -> String {
        build_set_cookie_header(
            &self.config.session_data_name,
            encrypted_data,
            &self.config,
            Some(self.config.data_max_age),
        )
    }

    /// Build "don't remember me" cookie (session-only)
    pub fn dont_remember(&self) -> String {
        build_set_cookie_header(
            &self.config.dont_remember_name,
            "true",
            &self.config,
            None,  // Session cookie
        )
    }

    /// Build cookie deletion headers
    pub fn delete_all(&self) -> Vec<String> {
        vec![
            build_set_cookie_header(&self.config.session_token_name, "", &self.config, Some(Duration::seconds(0))),
            build_set_cookie_header(&self.config.session_data_name, "", &self.config, Some(Duration::seconds(0))),
            build_set_cookie_header(&self.config.dont_remember_name, "", &self.config, Some(Duration::seconds(0))),
        ]
    }
}

/// Parse cookies from header
pub fn parse_cookies(cookie_header: &str) -> std::collections::HashMap<String, String> {
    let mut cookies = std::collections::HashMap::new();

    for cookie in cookie_header.split(';') {
        let cookie = cookie.trim();
        if let Some((name, value)) = cookie.split_once('=') {
            cookies.insert(name.to_string(), value.to_string());
        }
    }

    cookies
}
```

---

## Session Management

### Session Manager

```rust
// crates/better-auth-session/src/manager.rs

use better_auth_db::adapter::DatabaseAdapter;
use better_auth_types::{Session, User};
use crate::token::TokenGenerator;
use crate::cookie::{CookieConfig, SessionCookieBuilder};
use chrono::{Utc, Duration};

pub struct SessionManager {
    db: Box<dyn DatabaseAdapter>,
    token_generator: TokenGenerator,
    cookie_builder: SessionCookieBuilder,
    config: SessionManagerConfig,
}

pub struct SessionManagerConfig {
    pub cookie: CookieConfig,
    pub session_expiration: Duration,
    pub sliding_expiration: bool,
    pub absolute_expiration: Duration,
}

impl SessionManager {
    pub fn new(
        db: Box<dyn DatabaseAdapter>,
        token_generator: TokenGenerator,
        config: SessionManagerConfig,
    ) -> Self {
        Self {
            db,
            token_generator,
            cookie_builder: SessionCookieBuilder::new(config.cookie.clone()),
            config,
        }
    }

    /// Create a new session
    pub async fn create_session(
        &self,
        user: &User,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<SessionWithCookies, SessionError> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();

        let session = Session {
            id: session_id.clone(),
            user_id: user.id.clone(),
            token: uuid::Uuid::new_v4().to_string(),
            expires_at: (now + self.config.session_expiration).timestamp(),
            created_at: now.timestamp(),
            ip_address,
            user_agent,
            metadata: None,
            last_active_at: Some(now.timestamp()),
            refreshed_at: Some(now.timestamp()),
        };

        // Store in database
        self.db.create_session(&session).await?;

        // Generate token
        let token = self.token_generator.generate(user.id.clone(), session_id.clone())?;

        // Build cookies
        let cookies = SessionCookies {
            session_token: self.cookie_builder.session_token(&token),
            session_data: None,  // Could cache encrypted session data here
            dont_remember: None,
        };

        Ok(SessionWithCookies {
            session,
            user: user.clone(),
            token,
            cookies,
        })
    }

    /// Get and verify a session from token
    pub async fn get_session(&self, token: &str) -> Result<Option<SessionWithUser>, SessionError> {
        // Verify token signature and get claims
        let claims = match self.token_generator.verify(token) {
            Ok(c) => c,
            Err(e) => return Err(SessionError::InvalidToken(e)),
        };

        // Check if token is expired
        if claims.is_expired() {
            return Ok(None);
        }

        // Get session from database
        let session = match self.db.find_session_by_id(&claims.sid).await? {
            Some(s) => s,
            None => return Ok(None),  // Session revoked or not found
        };

        // Check session expiration
        if session.expires_at < Utc::now().timestamp() {
            return Ok(None);
        }

        // Get user
        let user = match self.db.find_user_by_id(&session.user_id).await? {
            Some(u) => u,
            None => return Ok(None),  // User deleted
        };

        // Sliding expiration: extend session if active
        if self.config.sliding_expiration {
            let now = Utc::now();
            let session_expires = chrono::DateTime::from_timestamp(session.expires_at, 0).unwrap();

            if session_expires < now + self.config.session_expiration {
                // Extend expiration
                let new_expires = (now + self.config.session_expiration).timestamp();
                self.db.update_session_expiration(&session.id, new_expires).await?;
            }

            // Update last active
            self.db.update_session_activity(&session.id, now.timestamp()).await?;
        }

        Ok(Some(SessionWithUser { session, user }))
    }

    /// Revoke a single session
    pub async fn revoke_session(&self, session_id: &str) -> Result<(), SessionError> {
        self.db.delete_session(session_id).await?;
        Ok(())
    }

    /// Revoke all sessions for a user
    pub async fn revoke_all_sessions(&self, user_id: &str) -> Result<u32, SessionError> {
        let count = self.db.delete_all_user_sessions(user_id).await?;
        Ok(count)
    }

    /// Revoke all sessions except current
    pub async fn revoke_other_sessions(
        &self,
        user_id: &str,
        exclude_session: &str,
    ) -> Result<u32, SessionError> {
        let count = self.db.delete_other_user_sessions(user_id, exclude_session).await?;
        Ok(count)
    }

    /// Refresh session token (rotate token, keep session)
    pub async fn refresh_token(&self, session: &Session) -> Result<String, SessionError> {
        // Generate new token
        let new_token = self.token_generator.generate(
            session.user_id.clone(),
            session.id.clone()
        )?;

        // Update session token in database
        self.db.update_session_token(&session.id, &new_token).await?;

        Ok(new_token)
    }

    /// Build cookie headers for response
    pub fn build_cookies(&self, token: &str, dont_remember: bool) -> Vec<String> {
        let mut cookies = vec![self.cookie_builder.session_token(token)];

        if dont_remember {
            cookies.push(self.cookie_builder.dont_remember());
        }

        cookies
    }

    /// Build cookie deletion headers
    pub fn build_delete_cookies(&self) -> Vec<String> {
        self.cookie_builder.delete_all()
    }
}

pub struct SessionWithCookies {
    pub session: Session,
    pub user: User,
    pub token: String,
    pub cookies: SessionCookies,
}

pub struct SessionCookies {
    pub session_token: String,
    pub session_data: Option<String>,
    pub dont_remember: Option<String>,
}

pub struct SessionWithUser {
    pub session: Session,
    pub user: User,
}

#[derive(Error, Debug)]
pub enum SessionError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] better_auth_db::DbError),

    #[error("Token error: {0}")]
    TokenError(#[from] crate::token::TokenError),

    #[error("Invalid token: {0}")]
    InvalidToken(crate::token::TokenError),

    #[error("Session not found")]
    NotFound,

    #[error("Session expired")]
    Expired,

    #[error("User not found")]
    UserNotFound,
}
```

---

## Security Considerations

### Token Security Checklist

- [ ] Use minimum 256-bit (32 byte) secrets
- [ ] Rotate secrets periodically
- [ ] Use constant-time comparison for token verification
- [ ] Include expiration in all tokens
- [ ] Include issuer and audience claims for JWT
- [ ] Use `jti` claim for token revocation tracking
- [ ] Implement token refresh with rotation

### Cookie Security Checklist

- [ ] Always use `HttpOnly` flag
- [ ] Always use `Secure` flag in production
- [ ] Use `SameSite=Lax` or `SameSite=Strict`
- [ ] Set appropriate `Max-Age` (not too long)
- [ ] Use unique cookie names to avoid conflicts
- [ ] Sign all cookies with HMAC
- [ ] Consider encrypting session data cookie

### Session Security Checklist

- [ ] Store sessions in database for revocation capability
- [ ] Implement sliding expiration for active users
- [ ] Implement absolute expiration for security
- [ ] Track IP address and user agent for anomaly detection
- [ ] Provide "sign out everywhere" functionality
- [ ] Invalidate sessions on password change
- [ ] Rate limit session creation

### Secret Rotation

```rust
// Secret rotation support
pub struct MultiKeySigner {
    current_key_id: u32,
    keys: std::collections::HashMap<u32, SecretKey>,
}

impl MultiKeySigner {
    pub fn new(current_key_id: u32, keys: Vec<(u32, Vec<u8>)>) -> Self {
        let keys = keys
            .into_iter()
            .map(|(id, secret)| (id, SecretKey::from_bytes(&secret)))
            .collect();

        Self {
            current_key_id,
            keys,
        }
    }

    /// Sign with current key
    pub fn sign(&self, claims: SessionClaims) -> Result<String, JwtError> {
        let key = self.keys.get(&self.current_key_id)
            .ok_or(JwtError::SigningFailed)?;

        let jwt_claims = Claims::with_custom_claims(claims, Duration::hours(24));
        key.authenticate(jwt_claims)
            .map_err(|_| JwtError::SigningFailed)
    }

    /// Verify with any key (supports old tokens)
    pub fn verify(&self, token: &str) -> Result<SessionClaims, JwtError> {
        // Try current key first
        if let Some(key) = self.keys.get(&self.current_key_id) {
            if let Ok(claims) = key.verify::<SessionClaims>(token) {
                return Ok(claims.custom);
            }
        }

        // Try other keys (for backward compatibility)
        for (id, key) in &self.keys {
            if *id == self.current_key_id {
                continue;  // Already tried
            }

            if let Ok(claims) = key.verify::<SessionClaims>(token) {
                return Ok(claims.custom);
            }
        }

        Err(JwtError::VerificationFailed)
    }
}
```

---

## Migration from TypeScript

```typescript
// TypeScript reference (existing better-auth)
const sessionConfig = {
  expiresIn: 60 * 60 * 24 * 7,  // 7 days
  updateAge: 60 * 60 * 24,      // 1 day (sliding window)
  cookie: {
    name: 'better-auth.session',
    secure: true,
    httpOnly: true,
    sameSite: 'lax' as const,
  }
};

// Rust equivalent
use chrono::Duration;

let config = SessionManagerConfig {
    cookie: CookieConfig {
        session_token_name: "better-auth.session-token".to_string(),
        secure: true,
        http_only: true,
        same_site: SameSite::Lax,
        ..Default::default()
    },
    session_expiration: Duration::days(7),
    sliding_expiration: true,
    absolute_expiration: Duration::days(30),
};
```
