# CID Router API Utils Deep Dive

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.ContentAddressing/cid-router/crates/api-utils/`

---

## Overview

The `api-utils` crate provides shared error handling utilities for the CID Router HTTP API server. It defines a consistent error response format and type aliases that are used across all API endpoints.

**Key Responsibilities:**
- Standardized error response structure
- HTTP status code mapping
- Debug formatting with callstack support
- Integration with Axum's `IntoResponse` trait
- Support for external API error context

---

## Module Structure

```
api-utils/
├── src/
│   ├── lib.rs          # Module exports
│   ├── error.rs        # ApiError type definition
│   └── result.rs       # ApiResult type alias
└── Cargo.toml
```

---

## Type Definitions

### ApiResult

A type alias for `Result` with `ApiError` as the error type:

```rust
pub type ApiResult<T> = Result<T, ApiError>;
```

**Usage in endpoints:**

```rust
pub async fn get_data(
    Path(cid): Path<String>,
    State(ctx): State<Arc<Context>>,
) -> ApiResult<Response> {
    // ...
}
```

---

### ApiError

The main error type for all API errors:

```rust
#[derive(Debug)]
pub struct ApiError {
    status_code: StatusCode,
    body: ApiErrorBody,
}
```

**Fields:**
| Field | Type | Description |
|-------|------|-------------|
| `status_code` | `StatusCode` | HTTP status code (4xx, 5xx) |
| `body` | `ApiErrorBody` | JSON error body |

---

### ApiErrorBody

The JSON structure returned in error responses:

```rust
#[derive(Debug, Deserialize, Serialize)]
pub struct ApiErrorBody {
    error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    callstack: Option<Callstack>,
}
```

**Example JSON Response:**

```json
{
  "error": "No route found for CID",
  "callstack": "Internal error details..."
}
```

---

### Callstack

Provides debug context for errors:

```rust
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Callstack {
    Internal(String),
    External {
        url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<Value>,
    },
}
```

**Variants:**

| Variant | When to Use |
|---------|-------------|
| `Internal(String)` | Internal server error with backtrace |
| `External { url, error }` | Error from external API call |

---

## Constructors

### new() - Basic Error

Create a simple error with status code and message:

```rust
impl ApiError {
    pub fn new(status_code: StatusCode, error: impl Into<String>) -> Self {
        let error = error.into();
        let callstack = None;

        Self {
            status_code,
            body: ApiErrorBody { error, callstack },
        }
    }
}
```

**Usage:**

```rust
// 404 Not Found
return Err(ApiError::new(StatusCode::NOT_FOUND, "No route found for CID"));

// 400 Bad Request
return Err(ApiError::new(StatusCode::BAD_REQUEST, "Invalid CID format"));

// 415 Unsupported Media Type
return Err(ApiError::new(StatusCode::UNSUPPORTED_MEDIA_TYPE, "Unsupported content-type"));
```

---

### new_with_external_error() - External API Error

Create an error that includes context from an external API call:

```rust
pub fn new_with_external_error(
    status_code: StatusCode,
    error: impl Into<String>,
    url: impl Into<String>,
    external_error: Option<Value>,
) -> Self {
    let error = error.into();
    let url = url.into();
    let callstack = Some(Callstack::External {
        url,
        error: external_error,
    });

    Self {
        status_code,
        body: ApiErrorBody { error, callstack },
    }
}
```

**Usage:**

```rust
// JWKS fetch failed
return Err(ApiError::new_with_external_error(
    StatusCode::SERVICE_UNAVAILABLE,
    "Failed to fetch JWKS",
    "https://auth.example.com/.well-known/jwks.json",
    Some(json!({"code": "ECONNREFUSED"})),
));
```

---

## From Trait Implementations

### From<anyhow::Error>

Automatically converts `anyhow::Error` to `ApiError` with backtrace:

```rust
impl<E> From<E> for ApiError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        let err = err.into();
        let status_code = StatusCode::INTERNAL_SERVER_ERROR;
        let error = err.to_string();
        let callstack = Some(Callstack::Internal(err.backtrace().to_string()));

        Self {
            status_code,
            body: ApiErrorBody { error, callstack },
        }
    }
}
```

**Usage:**

```rust
// Using ? operator with anyhow::Error
async fn handler() -> ApiResult<Json<Data>> {
    let data = some_anyhow_fn().await?;  // Auto-converted to ApiError
    Ok(Json(data))
}
```

**Key Behavior:**
- Always maps to `500 Internal Server Error`
- Includes full backtrace in `callstack` field
- Logs the error with full debug info

---

## IntoResponse Implementation

Integrates with Axum's response system:

```rust
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        log::error!("API error: {:#?}", self);

        let Self { status_code, body } = self;

        (
            status_code,
            serde_json::to_string(&body).unwrap_or("Unrepresentable error.".to_owned()),
        )
            .into_response()
    }
}
```

**Behavior:**
1. Logs the error with `log::error!`
2. Serializes `ApiErrorBody` to JSON
3. Returns tuple of `(StatusCode, JSON string)`

**Example HTTP Response:**

```
HTTP/1.1 404 Not Found
Content-Type: application/json

{
  "error": "No route found for CID",
  "callstack": "..."
}
```

---

## Common Error Patterns

### 400 Bad Request

```rust
// Invalid CID
let cid = Cid::from_str(&cid_str)
    .map_err(|e| ApiError::new(StatusCode::BAD_REQUEST, e.to_string()))?;

// Failed to read body
return Err(ApiError::new(
    StatusCode::BAD_REQUEST,
    "Failed to read request body",
));
```

### 401 Unauthorized

```rust
// Authentication failed
ctx.auth.service().await.authenticate(token).await
    .map_err(|e| ApiError::new(StatusCode::UNAUTHORIZED, e.to_string()))?;
```

### 404 Not Found

```rust
// No route found
Err(ApiError::new(StatusCode::NOT_FOUND, "No route found for CID"))
```

### 415 Unsupported Media Type

```rust
// Invalid content-type
return Err(ApiError::new(
    StatusCode::UNSUPPORTED_MEDIA_TYPE,
    "Unsupported content-type",
));
```

### 503 Service Unavailable

```rust
// No writers available
return Err(ApiError::new(
    StatusCode::SERVICE_UNAVAILABLE,
    "No eligible writers found for CID",
));
```

### 500 Internal Server Error

```rust
// Using ? with anyhow::Error - automatic conversion
let route = ctx.core.db().routes_for_cid(cid).await?;  // Auto-converted
```

---

## Debug Formatting

The `Callstack` enum has a custom `Debug` implementation:

```rust
impl std::fmt::Debug for Callstack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Callstack::Internal(inner) => write!(f, "{}", inner),
            Callstack::External { url, error } => {
                write!(f, "url: {}\n error: {:?}", url, error)
            }
        }
    }
}
```

**Output Examples:**

```
// Internal error
"at src/api/v1/data.rs:42
   <some::Error as core::convert::From<...>>::from
   ..."

// External error
"url: https://auth.example.com/.well-known/jwks.json
 error: Object {"code": String("ECONNREFUSED")}
```

---

## Usage in Endpoints

### Example: Complete Handler

```rust
use api_utils::{ApiError, ApiResult};
use axum::{extract::State, Json};

pub async fn get_routes(
    Path(cid): Path<String>,
    State(ctx): State<Arc<Context>>,
) -> ApiResult<Json<RoutesResponse>> {
    // Parse CID - returns 400 on error
    let cid = Cid::from_str(&cid)
        .map_err(|e| ApiError::new(StatusCode::BAD_REQUEST, e.to_string()))?;

    // Query database - uses From<anyhow::Error>
    let routes = ctx.core.db().routes_for_cid(cid).await?;

    // Build response
    Ok(Json(RoutesResponse { routes }))
}
```

### Example: Error Propagation

```rust
pub async fn handler(
    State(ctx): State<Arc<Context>>,
) -> ApiResult<Json<Data>> {
    // Multiple ? operations - all auto-converted
    let data = fetch_data().await?;
    let validated = validate(data)?;
    let stored = ctx.db.insert(validated).await?;

    Ok(Json(stored))
}
```

---

## Best Practices

### 1. Use Specific Status Codes

```rust
// GOOD: Specific status code
Err(ApiError::new(StatusCode::UNSUPPORTED_MEDIA_TYPE, "Unsupported content-type"))

// BAD: Generic 500 for client error
Err(ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Bad content-type"))
```

### 2. Provide Clear Error Messages

```rust
// GOOD: Clear message
Err(ApiError::new(StatusCode::NOT_FOUND, "No route found for CID"))

// BAD: Vague message
Err(ApiError::new(StatusCode::NOT_FOUND, "Error"))
```

### 3. Use ? for Internal Errors

```rust
// GOOD: Let From trait handle internal errors
let data = ctx.db.get(id).await?;

// GOOD: Custom message for client errors
let cid = Cid::from_str(&cid_str)
    .map_err(|e| ApiError::new(StatusCode::BAD_REQUEST, e.to_string()))?;
```

### 4. Use External Error Context for API Calls

```rust
// GOOD: Include URL and error from external call
let response = reqwest::get(&url).await
    .map_err(|e| ApiError::new_with_external_error(
        StatusCode::SERVICE_UNAVAILABLE,
        "Failed to fetch JWKS",
        &url,
        Some(json!({"error": e.to_string()})),
    ))?;
```

---

## Dependency Graph

```
api-utils (no workspace dependencies)
    │
    ├── axum              # HTTP framework
    ├── serde             # Serialization
    ├── serde_json        # JSON handling
    ├── log               # Logging
    └── anyhow            # Error context
    │
    └── Used by:
        └── cid-router-server
```

---

## Testing

### Unit Test Example

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    #[test]
    fn test_error_creation() {
        let error = ApiError::new(StatusCode::NOT_FOUND, "Not found");
        assert_eq!(error.status_code, StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_external_error_creation() {
        let error = ApiError::new_with_external_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "API error",
            "https://api.example.com",
            None,
        );
        assert!(matches!(error.body.callstack, Some(Callstack::External { .. })));
    }
}
```

---

## See Also

- [Server API Deep Dive](./cid-router-server-deep-dive.md)
- [Core Library Deep Dive](./cid-router-core-deep-dive.md)
- [Architecture Overview](./cid-router-architecture-deep-dive.md)
