---
title: "Authentication System Deep Dive"
subtitle: "AuthConfig types, resolution, and security patterns"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/wildcard-ai/05-authentication-system-deep-dive.md
prerequisites: 00-zero-to-ai-engineer.md
---

# Authentication System Deep Dive

## Overview

The authentication system supports multiple auth types for API integration:
1. API Key
2. Bearer Token
3. Basic Auth
4. OAuth 1.0
5. OAuth 2.0

---

## AuthConfig Types

### Enum Definition

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AuthConfig {
    #[serde(rename = "bearer")]
    Bearer(BearerAuth),

    #[serde(rename = "apiKey")]
    ApiKey(ApiKeyAuth),

    #[serde(rename = "basic")]
    Basic(BasicAuth),

    #[serde(rename = "oauth1")]
    OAuth1(OAuth1Auth),

    #[serde(rename = "oauth2")]
    OAuth2(OAuth2Auth),
}
```

---

## Bearer Authentication

### Structure

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BearerAuth {
    pub token: String,
}
```

### Usage

```rust
let auth = AuthConfig::Bearer(BearerAuth {
    token: "eyJhbGciOiJIUzI1NiIs...".to_string(),
});
```

### HTTP Header

```
Authorization: Bearer eyJhbGciOiJIUzI1NiIs...
```

### Application

```rust
fn add_bearer_auth(request: RequestBuilder, token: &str) -> RequestBuilder {
    request.header("Authorization", format!("Bearer {}", token))
}
```

---

## API Key Authentication

### Structure

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyAuth {
    #[serde(rename = "keyValue")]
    pub key_value: String,

    #[serde(rename = "keyName", skip_serializing_if = "Option::is_none")]
    pub key_name: Option<String>,

    #[serde(rename = "keyPrefix", skip_serializing_if = "Option::is_none")]
    pub key_prefix: Option<String>,
}
```

### Usage

```rust
// Simple API key
let auth = AuthConfig::ApiKey(ApiKeyAuth {
    key_value: "re_123456".to_string(),
    key_name: None,
    key_prefix: None,
});

// Custom header name
let auth = AuthConfig::ApiKey(ApiKeyAuth {
    key_value: "abc123".to_string(),
    key_name: Some("X-API-Key".to_string()),
    key_prefix: None,
});

// With prefix
let auth = AuthConfig::ApiKey(ApiKeyAuth {
    key_value: "secret".to_string(),
    key_name: Some("Authorization".to_string()),
    key_prefix: Some("Bearer".to_string()),
});
```

### HTTP Headers

```
# Default (X-API-Key)
X-API-Key: re_123456

# Custom header
X-Custom-API-Key: abc123

# With prefix
Authorization: Bearer secret
```

### Application

```rust
fn add_api_key_auth(
    request: RequestBuilder,
    key_value: &str,
    key_name: Option<&str>,
    key_prefix: Option<&str>,
) -> RequestBuilder {
    let header_name = key_name.unwrap_or("X-API-Key");

    let header_value = if let Some(prefix) = key_prefix {
        format!("{} {}", prefix, key_value)
    } else {
        key_value.to_string()
    };

    request.header(header_name, header_value)
}
```

---

## Basic Authentication

### Structure

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicAuth {
    pub credentials: BasicCredentials,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BasicCredentials {
    UserPass(UserPassCredentials),
    Base64(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPassCredentials {
    pub username: String,
    pub password: String,

    #[serde(default)]
    pub base64_encode: bool,
}
```

### Usage

```rust
// Username/password
let auth = AuthConfig::Basic(BasicAuth {
    credentials: BasicCredentials::UserPass(UserPassCredentials {
        username: "user".to_string(),
        password: "pass".to_string(),
        base64_encode: false,
    }),
});

// Pre-encoded
let auth = AuthConfig::Basic(BasicAuth {
    credentials: BasicCredentials::Base64("dXNlcjpwYXNz".to_string()),
});
```

### HTTP Header

```
# Username/password
Authorization: Basic dXNlcjpwYXNz

# Where: dXNlcjpwYXNz = base64("user:pass")
```

### Application

```rust
use base64::{Engine, engine::general_purpose};

fn add_basic_auth(
    request: RequestBuilder,
    credentials: &BasicCredentials,
) -> RequestBuilder {
    let auth_string = match credentials {
        BasicCredentials::UserPass(creds) => {
            let raw = format!("{}:{}", creds.username, creds.password);
            if creds.base64_encode {
                general_purpose::STANDARD.encode(&raw)
            } else {
                raw
            }
        }
        BasicCredentials::Base64(s) => s.clone(),
    };

    request.header("Authorization", format!("Basic {}", auth_string))
}
```

---

## OAuth 1.0 Authentication

### Structure

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth1Auth {
    pub consumer_key: String,
    pub consumer_secret: String,
    pub access_token: String,
    pub access_token_secret: String,
}
```

### Usage

```rust
let auth = AuthConfig::OAuth1(OAuth1Auth {
    consumer_key: "ck_123".to_string(),
    consumer_secret: "cs_456".to_string(),
    access_token: "at_789".to_string(),
    access_token_secret: "ats_012".to_string(),
});
```

### OAuth 1.0 Signature

```rust
use hmac::{Hmac, Mac};
use sha1::Sha1;
use base64::{Engine, engine::general_purpose};
use std::collections::HashMap;

type HmacSha1 = Hmac<Sha1>;

pub fn generate_oauth1_signature(
    method: &str,
    url: &str,
    params: &HashMap<String, String>,
    consumer_secret: &str,
    access_token_secret: &str,
) -> String {
    // Create base string
    let mut sorted_params: Vec<_> = params.iter().collect();
    sorted_params.sort_by_key(|(k, _)| *k);

    let param_string = sorted_params
        .iter()
        .map(|(k, v)| format!("{}={}", url_encode(k), url_encode(v)))
        .collect::<Vec<_>>()
        .join("&");

    let base_string = format!(
        "{}&{}&{}",
        method.to_uppercase(),
        url_encode(url),
        url_encode(&param_string)
    );

    // Create signing key
    let signing_key = format!("{}&{}", consumer_secret, access_token_secret);

    // Generate HMAC
    let mut mac = HmacSha1::new_from_slice(signing_key.as_bytes()).unwrap();
    mac.update(base_string.as_bytes());
    let result = mac.finalize();

    general_purpose::STANDARD.encode(result.into_bytes())
}
```

### OAuth 1.0 Header

```
Authorization: OAuth
  oauth_consumer_key="ck_123",
  oauth_token="at_789",
  oauth_signature_method="HMAC-SHA1",
  oauth_timestamp="1234567890",
  oauth_nonce="random_string",
  oauth_signature="generated_signature"
```

---

## OAuth 2.0 Authentication

### Structure

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2Auth {
    pub token: String,

    #[serde(rename = "tokenType", skip_serializing_if = "Option::is_none")]
    pub token_type: Option<String>,

    #[serde(rename = "refreshToken", skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,

    #[serde(rename = "expiresAt", skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub scopes: Option<Vec<String>>,
}
```

### Usage

```rust
let auth = AuthConfig::OAuth2(OAuth2Auth {
    token: "ya29.access_token".to_string(),
    token_type: Some("Bearer".to_string()),
    refresh_token: Some("refresh_token".to_string()),
    expires_at: Some(1234567890),
    scopes: Some(vec!["read".to_string(), "write".to_string()]),
});
```

### Token Refresh

```rust
pub async fn refresh_oauth2_token(
    client: &reqwest::Client,
    auth: &OAuth2Auth,
    client_id: &str,
    client_secret: &str,
) -> Result<OAuth2Auth, AuthError> {
    let response = client
        .post("https://oauth.provider.com/token")
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", auth.refresh_token.as_ref().unwrap()),
            ("client_id", client_id),
            ("client_secret", client_secret),
        ])
        .send()
        .await?;

    let new_auth: OAuth2Auth = response.json().await?;
    Ok(new_auth)
}
```

---

## Auth Resolution

### Resolution Function

```rust
pub fn resolve_auth(auth: &AuthConfig) -> ResolvedAuth {
    match auth {
        AuthConfig::Bearer(bearer) => ResolvedAuth::Bearer(bearer.token.clone()),
        AuthConfig::ApiKey(api_key) => ResolvedAuth::ApiKey {
            key: api_key.key_value.clone(),
            header: api_key.key_name.clone().unwrap_or_else(|| "X-API-Key".to_string()),
            prefix: api_key.key_prefix.clone(),
        },
        AuthConfig::Basic(basic) => ResolvedAuth::Basic(resolve_basic_credentials(&basic.credentials)),
        AuthConfig::OAuth1(oauth1) => ResolvedAuth::OAuth1(oauth1.clone()),
        AuthConfig::OAuth2(oauth2) => ResolvedAuth::OAuth2(oauth2.token.clone()),
    }
}

fn resolve_basic_credentials(credentials: &BasicCredentials) -> String {
    match credentials {
        BasicCredentials::UserPass(creds) => {
            let raw = format!("{}:{}", creds.username, creds.password);
            if creds.base64_encode {
                general_purpose::STANDARD.encode(&raw)
            } else {
                general_purpose::STANDARD.encode(&raw)
            }
        }
        BasicCredentials::Base64(s) => s.clone(),
    }
}
```

### Applying Auth to Request

```rust
pub fn apply_auth(
    request: RequestBuilder,
    auth: &ResolvedAuth,
    method: &str,
    url: &str,
) -> RequestBuilder {
    match auth {
        ResolvedAuth::Bearer(token) => {
            request.header("Authorization", format!("Bearer {}", token))
        }
        ResolvedAuth::ApiKey { key, header, prefix } => {
            let value = if let Some(p) = prefix {
                format!("{} {}", p, key)
            } else {
                key.clone()
            };
            request.header(header, value)
        }
        ResolvedAuth::Basic(encoded) => {
            request.header("Authorization", format!("Basic {}", encoded))
        }
        ResolvedAuth::OAuth1(oauth1) => {
            let signature = generate_oauth1_signature(method, url, oauth1);
            let auth_header = build_oauth1_header(oauth1, &signature);
            request.header("Authorization", auth_header)
        }
        ResolvedAuth::OAuth2(token) => {
            request.header("Authorization", format!("Bearer {}", token))
        }
    }
}
```

---

## Security Best Practices

### Secrets Management

```rust
use std::env;

pub fn load_auth_from_env() -> Result<AuthConfig, AuthError> {
    let auth_type = env::var("AUTH_TYPE")
        .map_err(|_| AuthError::MissingEnvVar("AUTH_TYPE"))?;

    match auth_type.as_str() {
        "bearer" => Ok(AuthConfig::Bearer(BearerAuth {
            token: env::var("BEARER_TOKEN")?,
        })),
        "apiKey" => Ok(AuthConfig::ApiKey(ApiKeyAuth {
            key_value: env::var("API_KEY")?,
            key_name: env::var("API_KEY_HEADER").ok(),
            key_prefix: None,
        })),
        "basic" => Ok(AuthConfig::Basic(BasicAuth {
            credentials: BasicCredentials::UserPass(UserPassCredentials {
                username: env::var("BASIC_USERNAME")?,
                password: env::var("BASIC_PASSWORD")?,
                base64_encode: false,
            }),
        })),
        _ => Err(AuthError::UnknownAuthType(auth_type)),
    }
}
```

### Token Storage

```rust
use zeroize::Zeroize;

pub struct SecureToken {
    data: Vec<u8>,
}

impl SecureToken {
    pub fn new(token: &str) -> Self {
        Self {
            data: token.as_bytes().to_vec(),
        }
    }

    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.data).unwrap()
    }
}

impl Drop for SecureToken {
    fn drop(&mut self) {
        self.data.zeroize();
    }
}
```

---

## Error Types

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Missing environment variable: {0}")]
    MissingEnvVar(&'static str),

    #[error("Unknown auth type: {0}")]
    UnknownAuthType(String),

    #[error("Invalid credentials: {0}")]
    InvalidCredentials(String),

    #[error("Token expired")]
    TokenExpired,

    #[error("OAuth error: {0}")]
    OAuthError(String),
}
```

---

## Testing

### Mock Auth

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_auth() -> AuthConfig {
        AuthConfig::ApiKey(ApiKeyAuth {
            key_value: "test_key".to_string(),
            key_name: Some("X-API-Key".to_string()),
            key_prefix: None,
        })
    }

    #[test]
    fn test_api_key_auth() {
        let auth = create_test_auth();
        let resolved = resolve_auth(&auth);

        match resolved {
            ResolvedAuth::ApiKey { key, header, .. } => {
                assert_eq!(key, "test_key");
                assert_eq!(header, "X-API-Key");
            }
            _ => panic!("Expected ApiKey auth"),
        }
    }
}
```

---

*This document covers the authentication system. See 06-integration-packages-deep-dive.md for integration details.*
