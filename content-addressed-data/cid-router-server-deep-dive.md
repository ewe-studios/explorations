# CID Router Server API Deep Dive

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.ContentAddressing/cid-router/server/`

---

## Overview

The CID Router server is an HTTP API layer built with Axum that exposes the content-addressed routing system to clients. It provides RESTful endpoints for:

- Uploading and downloading content by CID
- Listing and querying routes
- Server status and health checks
- JWT-based authentication (EQTYLab variant)

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    CID Router Server                         │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │              HTTP Layer (Axum)                        │   │
│  │  - Router configuration                               │   │
│  │  - Middleware (auth, CORS, logging)                   │   │
│  │  - OpenAPI/Swagger generation                         │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │              API v1 Endpoints                         │   │
│  │  - /v1/data/{cid}        (GET, POST)                 │   │
│  │  - /v1/routes            (GET)                       │   │
│  │  - /v1/routes/{cid}      (GET)                       │   │
│  │  - /v1/status            (GET)                       │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │              Cross-Cutting Concerns                   │   │
│  │  - Authentication (JWT/JWKS)                         │   │
│  │  - Configuration (TOML)                              │   │
│  │  - Context (shared state)                            │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

## Module Structure

```
server/
├── src/
│   ├── main.rs             # Entry point, CLI parsing
│   ├── lib.rs              # Library exports
│   ├── api.rs              # API module root
│   ├── api/v1.rs           # V1 API grouping
│   ├── api/v1/
│   │   ├── data.rs         # Data upload/download
│   │   ├── routes.rs       # Route listing/querying
│   │   └── status.rs       # Health/status endpoint
│   ├── auth.rs             # JWT authentication
│   ├── config.rs           # Server configuration
│   ├── context.rs          # Server context (shared state)
│   └── cli.rs              # CLI argument parsing
└── Cargo.toml
```

---

## Server Configuration

### Config Structure

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub port: u16,
    pub auth: Auth,
    pub providers: Vec<ProviderConfig>,
}
```

### Provider Configuration

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum ProviderConfig {
    Iroh(IrohCrpConfig),
    Azure(AzureContainerConfig),
}
```

### Example server.toml

```toml
port = 8080
auth = "none"

[[providers]]
type = "iroh"
path = "/var/lib/cid-router/blobs"
writeable = true

[[providers]]
type = "azure"
account = "mystorageaccount"
container = "mycontainer"
filter = "all"
```

---

## Authentication System

### Auth Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum Auth {
    #[default]
    None,                    // No authentication
    EqtyJwt(EqtyJwt),        // EQTYLab JWT with JWKS
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EqtyJwt {
    pub jwks_url: String,    // URL to fetch JWKS from
}
```

### AuthService Trait

```rust
#[async_trait]
pub trait AuthService: Send + Sync + Debug {
    async fn authenticate(&self, token: Option<String>) -> Result<()>;
}
```

### NoneAuth Implementation

```rust
#[derive(Debug)]
struct NoneAuth;

#[async_trait]
impl AuthService for NoneAuth {
    async fn authenticate(&self, _token: Option<String>) -> Result<()> {
        Ok(())  // Always succeeds
    }
}
```

### EqtyAuthClient - JWT with JWKS Caching

```rust
#[derive(Debug)]
struct EqtyAuthClient {
    url: String,
    cache: Arc<RwLock<Option<JwksCache>>>,
}

#[derive(Debug)]
struct JwksCache {
    jwks: Jwks,
    fetched_at: Instant,
    ttl: Duration,  // 1 hour
}
```

**JWKS Fetch Flow:**

```
1. Client sends request with JWT token
2. Extract 'kid' from token header
3. Check JWKS cache (valid for 1 hour)
4. If cache miss, fetch from jwks_url
5. Find matching key by kid
6. Decode and validate JWT signature
7. Validate claims (exp, iat, etc.)
```

**Key Code:**

```rust
async fn authenticate(&self, token: Option<String>) -> Result<()> {
    let token = token.ok_or(anyhow!("Token is missing"))?;

    let header = decode_header(&token)?;
    let kid = header.kid.ok_or(anyhow!("Token doesn't have a kid"))?;

    let jwks = self.get_jwks().await?;
    let jwk = Self::find_jwk(&jwks, &kid)
        .ok_or(anyhow!("No matching key found in JWKS"))?;

    let decoding_key = DecodingKey::from_rsa_components(&jwk.n, &jwk.e)?;
    let validation = Validation::new(Algorithm::RS256);

    let _token_data = decode::<Claims>(&token, &decoding_key, &validation)?;
    Ok(())
}
```

---

## API Endpoints

### GET /v1/data/{cid} - Download Content

**Purpose:** Stream raw bytes for a given CID

**Handler:**

```rust
pub async fn get_data(
    Path(cid): Path<String>,
    auth: Option<TypedHeader<Authorization<Bearer>>>,
    State(ctx): State<Arc<Context>>,
) -> ApiResult<Response> {
    // 1. Parse CID from path
    let cid = Cid::from_str(&cid)?;

    // 2. Query database for routes
    let routes = ctx.core.db().routes_for_cid(cid).await?;

    // 3. Authenticate request
    ctx.auth.service().await.authenticate(token).await?;

    // 4. Find provider and stream content
    for route in routes {
        if let Some(provider) = ctx.providers.iter()
            .find(|p| route.provider_type == p.provider_type())
        {
            if let Some(resolver) = provider.capabilities().route_resolver {
                let stream = resolver.get_bytes(&route, None).await?;

                // Convert to HTTP response body
                let body = StreamBody::new(
                    stream.map(|r| r.map(Frame::data).map_err(std::io::Error::other))
                );

                return Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header(CONTENT_TYPE, "application/octet-stream")
                    .body(Body::new(body))?);
            }
        }
    }

    Err(ApiError::new(StatusCode::NOT_FOUND, "No route found"))
}
```

**Response:**

| Status | Description |
|--------|-------------|
| 200 | Content stream (application/octet-stream) |
| 400 | Invalid CID format |
| 401 | Authentication failed |
| 404 | No route found for CID |
| 500 | Provider error |

---

### POST /v1/data - Upload Content

**Purpose:** Upload new content and receive its CID

**Handler:**

```rust
pub async fn create_data(
    auth: Option<TypedHeader<Authorization<Bearer>>>,
    content_type: Option<TypedHeader<ContentType>>,
    State(ctx): State<Arc<Context>>,
    body: Body,
) -> ApiResult<Json<CreateDataResponse>> {
    // 1. Authenticate
    ctx.auth.service().await.authenticate(token).await?;

    // 2. Determine codec from content-type
    let cid_type = match content_type.as_ref().map(|ct| ct.as_str()) {
        None => Codec::Raw,
        Some("application/octet-stream") => Codec::Raw,
        Some("application/vnd.ipld.dag-cbor") => Codec::DagCbor,
        _ => return Err(UNSUPPORTED_MEDIA_TYPE),
    };

    // 3. Read body into memory
    let mut buffer = BytesMut::new();
    while let Some(chunk) = body.into_data_stream().next().await {
        buffer.extend_from_slice(&chunk?);
    }
    let data = buffer.freeze();

    // 4. Compute BLAKE3 CID
    let hash = blake3::hash(&data);
    let cid = blake3_hash_to_cid(hash.into(), cid_type);

    // 5. Find eligible writers
    let writers = ctx.providers.iter()
        .filter(|p| p.provider_is_eligible_for_cid(&cid))
        .filter_map(|p| p.capabilities().blob_writer.map(|w| (p, w)))
        .collect::<Vec<_>>();

    if writers.is_empty() {
        return Err(SERVICE_UNAVAILABLE("No eligible writers"));
    }

    // 6. Write to providers (skip if already exists)
    for (crp, writer) in writers {
        if !existing_ids.contains(&crp.provider_id()) {
            writer.put_blob(None, &cid, &data).await?;

            // Create route record
            let route = Route::builder(crp)
                .cid(cid)
                .size(data.len() as u64)
                .url(cid.to_string())
                .build(&ctx.core)?;
            ctx.core.db().insert_route(&route).await?;
        }
    }

    Ok(Json(CreateDataResponse {
        cid: cid.to_string(),
        size: data.len(),
        location: format!("/v1/data/{}", cid),
    }))
}
```

**Response:**

```json
{
  "cid": "bafy83zj...",
  "size": 1048576,
  "location": "/v1/data/bafy83zj..."
}
```

**Status Codes:**

| Status | Description |
|--------|-------------|
| 200 | Content uploaded successfully |
| 400 | Failed to read request body |
| 401 | Authentication failed |
| 415 | Unsupported content-type |
| 503 | No eligible writers available |

---

### GET /v1/routes - List Routes

**Purpose:** List all routes with pagination

**Query Parameters:**

| Parameter | Default | Description |
|-----------|---------|-------------|
| direction | DESC | Sort direction (ASC/DESC) |
| offset | 0 | Pagination offset |
| limit | 100 | Max results to return |

**Handler:**

```rust
pub async fn list_routes(
    State(ctx): State<Arc<Context>>,
    query: Query<ListRoutesQuery>,
    auth: Option<TypedHeader<Authorization<Bearer>>>,
) -> ApiResult<Json<Vec<Route>>> {
    ctx.auth.service().await.authenticate(token).await?;

    let direction = Direction::from_str(&query.direction.unwrap_or("DESC"))?;
    let offset = query.offset.unwrap_or(0);
    let limit = query.limit.unwrap_or(100);

    let routes = ctx.core.db()
        .list_routes(OrderBy::CreatedAt(direction), offset, limit)
        .await?;

    let routes = routes.into_iter().map(Route::from).collect();
    Ok(Json(routes))
}
```

**Response Format:**

```json
[
  {
    "provider_id": "iroh",
    "type": "Iroh",
    "size": 1048576,
    "url": "bafy83zj...",
    "cid": "bafy83zj..."
  }
]
```

---

### GET /v1/routes/{cid} - Query Routes for CID

**Purpose:** Get all available routes for a specific CID

**Handler:**

```rust
pub async fn get_routes(
    Path(cid): Path<String>,
    auth: Option<TypedHeader<Authorization<Bearer>>>,
    State(ctx): State<Arc<Context>>,
) -> ApiResult<Json<RoutesResponse>> {
    ctx.auth.service().await.authenticate(token).await?;

    let cid = Cid::from_str(&cid)?;
    let routes = ctx.core.db().routes_for_cid(cid).await?;
    let routes = routes.into_iter().map(Route::from).collect();

    Ok(Json(RoutesResponse { routes }))
}
```

---

### GET /v1/status - Server Health

**Purpose:** Check server uptime and health

**Handler:**

```rust
pub async fn get_status(
    State(ctx): State<Arc<Context>>,
) -> ApiResult<Json<StatusResponse>> {
    let uptime = chrono::Utc::now().timestamp() - ctx.start_time;
    Ok(Json(StatusResponse { uptime }))
}
```

**Response:**

```json
{
  "uptime": 3600
}
```

---

## Server Context

### Shared State

```rust
pub struct Context {
    pub core: cid_router_core::context::Context,  // Core context
    pub auth: crate::auth::Auth,                   // Auth config
    pub providers: Vec<Arc<dyn Crp>>,              // Registered providers
    pub start_time: i64,                           // Server start timestamp
}
```

### Context Creation

```rust
impl Context {
    pub async fn new(
        core: cid_router_core::context::Context,
        auth: crate::auth::Auth,
        providers: Vec<Arc<dyn Crp>>,
    ) -> Self {
        Self {
            core,
            auth,
            providers,
            start_time: chrono::Utc::now().timestamp(),
        }
    }
}
```

---

## OpenAPI Documentation

The server uses the `utoipa` crate for automatic OpenAPI spec generation:

```rust
/// Get data for a CID
#[utoipa::path(
    get,
    path = "/v1/data/{cid}",
    tag = "/v1/data/{cid}",
    params(
        ("authorization" = Option<String>, Header)
    ),
    responses(
        (status = 200, description = "Success", content_type = "application/octet-stream"),
        (status = 404, description = "No route found")
    )
)]
pub async fn get_data(...) { ... }
```

**Generate OpenAPI Spec:**

```bash
cid-router openapi ./docs
```

---

## Error Handling

### ApiError Type

```rust
pub struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    pub fn new(status: StatusCode, message: impl ToString) -> Self {
        Self {
            status,
            message: message.to_string(),
        }
    }
}
```

### ApiResult Type Alias

```rust
pub type ApiResult<T> = Result<T, ApiError>;
```

### Error Conversion

```rust
// From anyhow::Error
impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, err)
    }
}

// From cid::Error
impl From<cid::Error> for ApiError {
    fn from(err: cid::Error) -> Self {
        Self::new(StatusCode::BAD_REQUEST, err)
    }
}
```

---

## Data Flow

### Upload Flow

```
POST /v1/data [binary]
       │
       ▼
┌─────────────────┐
│  Read body      │
│  (BytesMut)     │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Compute BLAKE3 │
│  Create CID     │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Filter writers │
│  (cid_filter)   │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  put_blob()     │
│  (each writer)  │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Insert route   │
│  (SQLite)       │
└────────┬────────┘
         │
         ▼
   { cid, size, location }
```

### Download Flow

```
GET /v1/data/{cid}
       │
       ▼
┌─────────────────┐
│  Parse CID      │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Query routes   │
│  (SQLite)       │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Find provider  │
│  (by type/id)   │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  get_bytes()    │
│  (streaming)    │
└────────┬────────┘
         │
         ▼
   HTTP 200 + body stream
```

---

## Running the Server

### CLI Entry Point

```rust
#[derive(Parser)]
struct Cli {
    #[arg(long, default_value = "~/.local/share/cid-router")]
    repo_path: String,

    #[arg(subcommand)]
    command: Commands,
}

enum Commands {
    Start,
    Openapi { output_dir: PathBuf },
}
```

### Start Command

```bash
# Default config location
cid-router start

# Custom repo path
cid-router start --repo-path /custom/path
```

### Configuration File

Location: `~/.local/share/cid-router/server.toml`

```toml
port = 8080
auth = "none"

[[providers]]
type = "iroh"
path = "/var/lib/cid-router/blobs"
writeable = true

[[providers]]
type = "azure"
account = "myaccount"
container = "mycontainer"
filter = { directory = "data/" }
```

---

## Security Considerations

### Authentication

- **None (default):** Use only for development/internal deployments
- **EqtyJwt:** Production-ready JWT with RS256 signing

### JWKS Caching

```rust
// Cache validity: 1 hour
ttl: Duration::from_secs(3600)
```

**Why cache?**
- Reduces latency (no network round-trip per request)
- Reduces load on auth server
- Handles transient network failures

### Content-Type Validation

Only these content-types are accepted for uploads:

| Content-Type | Codec |
|--------------|-------|
| (none) | Raw |
| application/octet-stream | Raw |
| application/vnd.ipld.dag-cbor | DagCbor |

---

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_data() {
        let ctx = setup_test_context().await;
        let response = get_data(
            Path("bafy...".to_string()),
            None,
            State(Arc::new(ctx)),
        ).await;

        assert_eq!(response.unwrap().status(), StatusCode::OK);
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_upload_download_roundtrip() {
    let server = spawn_test_server().await;
    let data = b"hello world";

    // Upload
    let cid = server.upload(data).await;

    // Download
    let downloaded = server.download(&cid).await;

    assert_eq!(data, downloaded.as_ref());
}
```

---

## Performance Considerations

### Streaming

- **Downloads:** Content is streamed in chunks, never fully loaded into memory
- **Uploads:** Currently buffered in memory (limitation for large files)

### Concurrent Requests

- Axum handles concurrent requests via tokio runtime
- Database connections are protected by `Arc<Mutex<Connection>>`
- JWKS cache uses `RwLock` for concurrent reads

### Future Improvements

1. **Streaming Uploads:** Support for large file uploads without full buffering
2. **Connection Pooling:** Use a pool for SQLite connections
3. **Rate Limiting:** Add request rate limiting per client
4. **Caching:** Add HTTP caching headers for immutable content

---

## Related Resources

- [Axum Documentation](https://docs.rs/axum/)
- [Utoipa (OpenAPI)](https://github.com/juhaku/utoipa)
- [jsonwebtoken crate](https://crates.io/crates/jsonwebtoken)
- [Headers crate](https://crates.io/crates/headers)

---

## See Also

- [Architecture Overview](./cid-router-architecture-deep-dive.md)
- [Core Library Deep Dive](./cid-router-core-deep-dive.md)
- [Iroh CRP Deep Dive](./cid-router-iroh-crp-deep-dive.md)
- [Azure CRP Deep Dive](./cid-router-azure-crp-deep-dive.md)
