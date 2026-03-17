---
type: reference
created: 2026-03-17
---

# OAuth 2.0 Implementation Reference

Complete reference for implementing OAuth 2.0 providers in the Better Auth WASM system.

## Table of Contents

1. [OAuth 2.0 Flows](#oauth-20-flows)
2. [Provider Implementation](#provider-implementation)
3. [Built-in Providers](#built-in-providers)
4. [Custom Providers](#custom-providers)
5. [PKCE Implementation](#pkce-implementation)

---

## OAuth 2.0 Flows

### Authorization Code Flow (with PKCE)

```
┌─────────┐                              ┌─────────┐
│  User   │                              │  Your  │
│ (Browser)│                             │  App   │
└────┬────┘                              └────┬────┘
     │                                        │
     │  1. Click "Sign in with Provider"      │
     ├───────────────────────────────────────►│
     │                                        │
     │                                        │ 2. Generate state + code_verifier
     │                                        │    Store in KV/D2
     │                                        │
     │  3. Redirect to authorization URL      │
     │◄───────────────────────────────────────┤
     │                                        │
     │  4. User authenticates with Provider   │
     │   ┌─────────────────────────────────┐  │
     │   │         Provider (Google,       │  │
     │   │         GitHub, etc.)           │  │
     │   └─────────────────────────────────┘  │
     │                                        │
     │  5. Redirect back with auth code       │
     ├───────────────────────────────────────►│
     │                                        │
     │                                        │ 6. Exchange code + verifier for tokens
     │                                        │    (server-to-server)
     │                                        │
     │                                        │ 7. Get user info
     │                                        │
     │                                        │ 8. Create/link account
     │                                        │    Create session
     │                                        │
     │  9. Redirect to app with session       │
     │◄───────────────────────────────────────┤
     │                                        │
```

### Flow Components

```rust
// crates/better-auth-oauth/src/flows/authorization_code.rs

use crate::provider::OAuthProvider;
use crate::pkce::{generate_code_verifier, generate_code_challenge};
use better_auth_crypto::generate_secure_random;
use better_auth_db::adapter::DatabaseAdapter;
use better_auth_types::{OAuthState, UserInfo, Account, AuthError};
use chrono::{Utc, Duration};

pub struct AuthorizationCodeFlow<P: OAuthProvider> {
    provider: P,
    db: Box<dyn DatabaseAdapter>,
    client_id: String,
    client_secret: String,
    redirect_uri: String,
}

impl<P: OAuthProvider> AuthorizationCodeFlow<P> {
    pub fn new(
        provider: P,
        db: Box<dyn DatabaseAdapter>,
        client_id: String,
        client_secret: String,
        redirect_uri: String,
    ) -> Self {
        Self {
            provider,
            db,
            client_id,
            client_secret,
            redirect_uri,
        }
    }

    /// Step 2: Create authorization URL
    pub async fn create_authorization_url(
        &self,
        scopes: Vec<String>,
    ) -> Result<AuthorizationUrlResult, AuthError> {
        // Generate cryptographically secure state
        let state = generate_secure_random(32)?;

        // Generate PKCE code verifier and challenge
        let code_verifier = generate_code_verifier()?;
        let code_challenge = generate_code_challenge(&code_verifier)?;

        // Store state and verifier (expires in 10 minutes)
        let expires_at = Utc::now() + Duration::minutes(10);
        self.store_oauth_state(&state, &code_verifier, expires_at).await?;

        // Build authorization URL
        let auth_url = self.provider.authorization_url(
            &self.redirect_uri,
            &state,
            &code_challenge,
            &scopes,
        );

        Ok(AuthorizationUrlResult {
            url: auth_url,
            state,
            code_verifier,
        })
    }

    /// Step 6: Handle callback and exchange code for tokens
    pub async fn handle_callback(
        &self,
        code: String,
        state: String,
    ) -> Result<TokenResponse, AuthError> {
        // Verify state
        let stored_state = self.get_oauth_state(&state).await?
            .ok_or(AuthError::InvalidState)?;

        // Validate state matches
        if stored_state.state != state {
            return Err(AuthError::InvalidState);
        }

        // Check expiration
        if stored_state.expires_at < Utc::now() {
            return Err(AuthError::StateExpired);
        }

        // Exchange authorization code for tokens
        let tokens = self.exchange_code(code, &stored_state.code_verifier).await?;

        // Clean up used state
        self.delete_oauth_state(&state).await?;

        Ok(tokens)
    }

    /// Step 7: Get user info from provider
    pub async fn get_user_info(
        &self,
        access_token: &str,
    ) -> Result<UserInfo, AuthError> {
        self.provider.get_user_info(access_token).await
    }

    /// Exchange authorization code for tokens
    async fn exchange_code(
        &self,
        code: String,
        code_verifier: &str,
    ) -> Result<TokenResponse, AuthError> {
        let client = reqwest::Client::new();

        let response = client
            .post(self.provider.token_endpoint())
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&[
                ("grant_type", "authorization_code"),
                ("code", &code),
                ("redirect_uri", &self.redirect_uri),
                ("code_verifier", code_verifier),
                ("client_id", &self.client_id),
                ("client_secret", &self.client_secret),
            ])
            .send()
            .await
            .map_err(|e| AuthError::OAuthProviderError(e.to_string()))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(AuthError::OAuthProviderError(error));
        }

        let tokens: TokenResponse = response
            .json()
            .await
            .map_err(|e| AuthError::OAuthProviderError(e.to_string()))?;

        Ok(tokens)
    }

    async fn store_oauth_state(
        &self,
        state: &str,
        code_verifier: &str,
        expires_at: chrono::DateTime<Utc>,
    ) -> Result<(), AuthError> {
        use better_auth_types::OAuthState;

        let oauth_state = OAuthState {
            id: uuid::Uuid::new_v4().to_string(),
            state: state.to_string(),
            code_verifier: code_verifier.to_string(),
            expires_at: expires_at.timestamp(),
            created_at: Utc::now().timestamp(),
        };

        self.db.create_oauth_state(&oauth_state).await?;
        Ok(())
    }

    async fn get_oauth_state(
        &self,
        state: &str,
    ) -> Result<Option<OAuthState>, AuthError> {
        self.db.find_oauth_state(state).await
    }

    async fn delete_oauth_state(&self, state: &str) -> Result<(), AuthError> {
        self.db.delete_oauth_state(state).await
    }
}

pub struct AuthorizationUrlResult {
    pub url: String,
    pub state: String,
    pub code_verifier: String,
}

pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: Option<i64>,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
}
```

---

## Provider Implementation

### OAuth Provider Trait

```rust
// crates/better-auth-oauth/src/provider.rs

use async_trait::async_trait;
use better_auth_types::{UserInfo, AuthError};

/// OAuth provider trait
#[async_trait]
pub trait OAuthProvider: Send + Sync {
    /// Provider identifier (e.g., "github", "google")
    fn id(&self) -> &'static str;

    /// Authorization URL for this provider
    fn authorization_url(
        &self,
        redirect_uri: &str,
        state: &str,
        code_challenge: &str,
        scopes: &[String],
    ) -> String;

    /// Token endpoint URL
    fn token_endpoint(&self) -> &str;

    /// User info endpoint URL
    fn user_info_endpoint(&self) -> &str;

    /// Get user info from access token
    async fn get_user_info(&self, access_token: &str) -> Result<UserInfo, AuthError>;

    /// Optional: Refresh token endpoint
    fn refresh_token_endpoint(&self) -> Option<&str> {
        None
    }
}

/// User info returned from OAuth providers
#[derive(Debug, Clone)]
pub struct UserInfo {
    pub provider_id: String,
    pub provider_email: String,
    pub provider_name: Option<String>,
    pub provider_avatar: Option<String>,
    pub email_verified: bool,
}
```

---

## Built-in Providers

### GitHub Provider

```rust
// crates/better-auth-oauth/src/providers/github.rs

use super::OAuthProvider;
use async_trait::async_trait;
use better_auth_types::{UserInfo, AuthError};
use serde::Deserialize;

pub struct GitHubProvider {
    client_id: String,
    client_secret: String,
}

impl GitHubProvider {
    pub fn new(client_id: String, client_secret: String) -> Self {
        Self {
            client_id,
            client_secret,
        }
    }
}

#[async_trait]
impl OAuthProvider for GitHubProvider {
    fn id(&self) -> &'static str {
        "github"
    }

    fn authorization_url(
        &self,
        redirect_uri: &str,
        state: &str,
        code_challenge: &str,
        scopes: &[String],
    ) -> String {
        let scope = scopes.join(" ");
        format!(
            "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&state={}&code_challenge={}&scope={}",
            urlencoding::encode(&self.client_id),
            urlencoding::encode(redirect_uri),
            urlencoding::encode(state),
            urlencoding::encode(code_challenge),
            urlencoding::encode(&scope)
        )
    }

    fn token_endpoint(&self) -> &str {
        "https://github.com/login/oauth/access_token"
    }

    fn user_info_endpoint(&self) -> &str {
        "https://api.github.com/user"
    }

    async fn get_user_info(&self, access_token: &str) -> Result<UserInfo, AuthError> {
        let client = reqwest::Client::new();

        // Get user profile
        let profile_response = client
            .get(self.user_info_endpoint())
            .header("Authorization", format!("Bearer {}", access_token))
            .header("User-Agent", "better-auth")
            .send()
            .await
            .map_err(|e| AuthError::OAuthProviderError(e.to_string()))?;

        if !profile_response.status().is_success() {
            return Err(AuthError::OAuthProviderError(
                "Failed to get GitHub profile".to_string()
            ));
        }

        let profile: GitHubUser = profile_response
            .json()
            .await
            .map_err(|e| AuthError::OAuthProviderError(e.to_string()))?;

        // Get user emails (GitHub requires separate call)
        let emails_response = client
            .get("https://api.github.com/user/emails")
            .header("Authorization", format!("Bearer {}", access_token))
            .header("User-Agent", "better-auth")
            .send()
            .await
            .map_err(|e| AuthError::OAuthProviderError(e.to_string()))?;

        let emails: Vec<GitHubEmail> = emails_response
            .json()
            .await
            .unwrap_or_default();

        // Find primary email
        let primary_email = emails
            .iter()
            .find(|e| e.primary)
            .or_else(|| emails.first())
            .ok_or_else(|| AuthError::OAuthProviderError("No email found".to_string()))?;

        Ok(UserInfo {
            provider_id: profile.id.to_string(),
            provider_email: primary_email.email.clone(),
            provider_name: Some(profile.name.unwrap_or(profile.login)),
            provider_avatar: profile.avatar_url,
            email_verified: primary_email.verified,
        })
    }
}

#[derive(Deserialize)]
struct GitHubUser {
    id: u64,
    login: String,
    name: Option<String>,
    email: Option<String>,
    avatar_url: String,
}

#[derive(Deserialize)]
struct GitHubEmail {
    email: String,
    primary: bool,
    verified: bool,
}
```

### Google Provider

```rust
// crates/better-auth-oauth/src/providers/google.rs

use super::OAuthProvider;
use async_trait::async_trait;
use better_auth_types::{UserInfo, AuthError};
use serde::Deserialize;

pub struct GoogleProvider {
    client_id: String,
    client_secret: String,
}

impl GoogleProvider {
    pub fn new(client_id: String, client_secret: String) -> Self {
        Self {
            client_id,
            client_secret,
        }
    }
}

#[async_trait]
impl OAuthProvider for GoogleProvider {
    fn id(&self) -> &'static str {
        "google"
    }

    fn authorization_url(
        &self,
        redirect_uri: &str,
        state: &str,
        code_challenge: &str,
        scopes: &[String],
    ) -> String {
        let scope = scopes.join(" ");
        format!(
            "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&state={}&code_challenge={}&code_challenge_method=S256&response_type=code&scope={}",
            urlencoding::encode(&self.client_id),
            urlencoding::encode(redirect_uri),
            urlencoding::encode(state),
            urlencoding::encode(code_challenge),
            urlencoding::encode(&scope)
        )
    }

    fn token_endpoint(&self) -> &str {
        "https://oauth2.googleapis.com/token"
    }

    fn user_info_endpoint(&self) -> &str {
        "https://www.googleapis.com/oauth2/v2/userinfo"
    }

    async fn get_user_info(&self, access_token: &str) -> Result<UserInfo, AuthError> {
        let client = reqwest::Client::new();

        let response = client
            .get(self.user_info_endpoint())
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await
            .map_err(|e| AuthError::OAuthProviderError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(AuthError::OAuthProviderError(
                "Failed to get Google profile".to_string()
            ));
        }

        let profile: GoogleUser = response
            .json()
            .await
            .map_err(|e| AuthError::OAuthProviderError(e.to_string()))?;

        Ok(UserInfo {
            provider_id: profile.id,
            provider_email: profile.email,
            provider_name: Some(profile.name),
            provider_avatar: Some(profile.picture),
            email_verified: profile.verified_email,
        })
    }
}

#[derive(Deserialize)]
struct GoogleUser {
    id: String,
    email: String,
    name: String,
    picture: String,
    verified_email: bool,
}
```

---

## Custom Providers

### Adding a Custom OIDC Provider

```rust
// crates/better-auth-oauth/src/providers/generic.rs

use super::OAuthProvider;
use async_trait::async_trait;
use better_auth_types::{UserInfo, AuthError};
use serde::Deserialize;

/// Generic OIDC provider configuration
pub struct GenericOidcProvider {
    id: String,
    issuer: String,
    client_id: String,
    client_secret: String,
    authorization_endpoint: String,
    token_endpoint: String,
    user_info_endpoint: String,
    scopes: Vec<String>,
}

impl GenericOidcProvider {
    pub fn new(
        id: String,
        issuer: String,
        client_id: String,
        client_secret: String,
    ) -> Result<Self, AuthError> {
        // Discover OIDC endpoints
        let config_url = format!("{}/.well-known/openid-configuration", issuer.trim_end_matches('/'));

        // In production, fetch and parse OIDC discovery document
        // For now, use defaults
        Ok(Self {
            id,
            issuer,
            client_id,
            client_secret,
            authorization_endpoint: format!("{}/authorize", issuer),
            token_endpoint: format!("{}/token", issuer),
            user_info_endpoint: format!("{}/userinfo", issuer),
            scopes: vec!["openid".to_string(), "email".to_string(), "profile".to_string()],
        })
    }

    pub fn with_scopes(mut self, scopes: Vec<String>) -> Self {
        self.scopes = scopes;
        self
    }
}

#[async_trait]
impl OAuthProvider for GenericOidcProvider {
    fn id(&self) -> &'static str {
        &self.id
    }

    fn authorization_url(
        &self,
        redirect_uri: &str,
        state: &str,
        code_challenge: &str,
        scopes: &[String],
    ) -> String {
        let scope = if scopes.is_empty() {
            self.scopes.join(" ")
        } else {
            scopes.join(" ")
        };

        format!(
            "{}?client_id={}&redirect_uri={}&state={}&code_challenge={}&code_challenge_method=S256&response_type=code&scope={}",
            self.authorization_endpoint,
            urlencoding::encode(&self.client_id),
            urlencoding::encode(redirect_uri),
            urlencoding::encode(state),
            urlencoding::encode(code_challenge),
            urlencoding::encode(&scope)
        )
    }

    fn token_endpoint(&self) -> &str {
        &self.token_endpoint
    }

    fn user_info_endpoint(&self) -> &str {
        &self.user_info_endpoint
    }

    async fn get_user_info(&self, access_token: &str) -> Result<UserInfo, AuthError> {
        let client = reqwest::Client::new();

        let response = client
            .get(self.user_info_endpoint())
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await
            .map_err(|e| AuthError::OAuthProviderError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(AuthError::OAuthProviderError(
                "Failed to get user info".to_string()
            ));
        }

        let profile: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AuthError::OAuthProviderError(e.to_string()))?;

        Ok(UserInfo {
            provider_id: profile["sub"].as_str().unwrap_or("").to_string(),
            provider_email: profile["email"].as_str().unwrap_or("").to_string(),
            provider_name: profile["name"].as_str().map(String::from),
            provider_avatar: profile["picture"].as_str().map(String::from),
            email_verified: profile["email_verified"].as_bool().unwrap_or(false),
        })
    }
}
```

---

## PKCE Implementation

```rust
// crates/better-auth-oauth/src/pkce.rs

use better_auth_crypto::generate_secure_random;
use sha2::{Sha256, Digest};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PkceError {
    #[error("Failed to generate code verifier")]
    GenerationFailed,
}

/// Generate a PKCE code verifier (43-128 characters)
pub fn generate_code_verifier() -> Result<String, PkceError> {
    // Generate 32 bytes of random data and base64url encode
    let random_bytes = generate_secure_random(32)
        .map_err(|_| PkceError::GenerationFailed)?;

    // Base64url encode (URL-safe, no padding)
    let verifier = base64_url::encode(random_bytes.as_bytes());

    // Ensure it's in the valid range (43-128 chars)
    if verifier.len() < 43 || verifier.len() > 128 {
        return Err(PkceError::GenerationFailed);
    }

    Ok(verifier)
}

/// Generate a PKCE code challenge from verifier (SHA256)
pub fn generate_code_challenge(code_verifier: &str) -> Result<String, PkceError> {
    let digest = Sha256::digest(code_verifier.as_bytes());
    Ok(base64_url::encode(&digest))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_verifier_length() {
        let verifier = generate_code_verifier().unwrap();
        assert!(verifier.len() >= 43);
        assert!(verifier.len() <= 128);
    }

    #[test]
    fn test_challenge_generation() {
        let verifier = generate_code_verifier().unwrap();
        let challenge = generate_code_challenge(&verifier).unwrap();
        // SHA256 produces 32 bytes, base64url encoded = 43 chars
        assert_eq!(challenge.len(), 43);
    }
}
```

---

## OAuth Database Schema

```sql
-- OAuth Accounts table
CREATE TABLE oauth_accounts (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider_id TEXT NOT NULL,
    provider_account_id TEXT NOT NULL,
    access_token TEXT,
    refresh_token TEXT,
    expires_at INTEGER,
    scope TEXT,
    token_type TEXT,
    id_token TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE(provider_id, provider_account_id)
);

-- OAuth State storage (for PKCE flow)
CREATE TABLE oauth_state (
    id TEXT PRIMARY KEY,
    state TEXT UNIQUE NOT NULL,
    code_verifier TEXT NOT NULL,
    expires_at INTEGER NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE INDEX idx_oauth_accounts_user ON oauth_accounts(user_id);
CREATE INDEX idx_oauth_accounts_provider ON oauth_accounts(provider_id, provider_account_id);
CREATE INDEX idx_oauth_state_state ON oauth_state(state);
```

---

## Usage Example

```rust
use better_auth_oauth::{
    AuthorizationCodeFlow,
    providers::{GitHubProvider, GoogleProvider},
};
use better_auth_db::d2::D2Adapter;

// Initialize provider
let github = GitHubProvider::new(
    env!("GITHUB_CLIENT_ID").to_string(),
    env!("GITHUB_CLIENT_SECRET").to_string(),
);

// Create OAuth flow
let oauth = AuthorizationCodeFlow::new(
    github,
    Box::new(db),
    env!("GITHUB_CLIENT_ID").to_string(),
    env!("GITHUB_CLIENT_SECRET").to_string(),
    "https://auth.example.com/auth/oauth/github/callback".to_string(),
);

// Start flow
let auth_url_result = oauth.create_authorization_url(vec![
    "user:email".to_string(),
]).await?;

// Redirect user to auth_url_result.url

// Handle callback
let tokens = oauth.handle_callback(code, state).await?;
let user_info = oauth.get_user_info(&tokens.access_token).await?;

// Link or create account...
```
