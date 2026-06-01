---
title: rauthy Architecture
prev: 00-overview.md
next: 02-authentication.md
---

# rauthy Architecture

System architecture and component design.

## High-Level Architecture

```mermaid
flowchart TB
    subgraph Clients["Clients"]
        WEB[Web Browser]
        APP[Mobile App]
        CLI[CLI Tool]
    end

    subgraph Rauthy["rauthy Server"]
        HTTP[HTTP Server]
        OIDC[OIDC/OAuth2 Handler]
        AUTH[Auth Handler]
        WEBAUTHN[FIDO2/WebAuthn]
        API[Admin API]
    end

    subgraph Services["Internal Services"]
        TOKEN[Token Service]
        USER[User Service]
        CLIENT[Client Service]
        SESSION[Session Service]
        CACHE[Cache Layer]
    end

    subgraph Data["Data Layer"]
        HIQLITE[Hiqlite]
        POSTGRES[Postgres]
    end

    Clients --> HTTP
    HTTP --> OIDC
    HTTP --> AUTH
    HTTP --> WEBAUTHN
    HTTP --> API
    
    OIDC --> TOKEN
    OIDC --> USER
    AUTH --> USER
    WEBAUTHN --> USER
    API --> CLIENT
    
    TOKEN --> CACHE
    USER --> CACHE
    CLIENT --> CACHE
    SESSION --> CACHE
    
    CACHE --> HIQLITE
    CACHE --> POSTGRES
```

## Source Structure

```
rauthy/src/
├── api/                # HTTP API handlers
├── api_types/          # API type definitions
├── bin/                # Binary entry points
├── common/             # Common utilities
├── data/               # Data models
├── error/              # Error handling
├── jwt/                # JWT handling
├── macros/             # Rust macros
├── middlewares/        # HTTP middlewares
├── notify/             # Notification system
├── schedulers/         # Background tasks
├── service/            # Business logic
└── wasm-modules/       # WebAssembly modules
```

## Component Breakdown

### API Layer

**Location:** `src/api/`

HTTP request handlers:

| Module | Purpose | Endpoints |
|--------|---------|-----------|
| `auth.rs` | Authentication | `/auth`, `/login` |
| `oidc.rs` | OIDC endpoints | `/authorize`, `/token` |
| `users.rs` | User API | `/users/*` |
| `clients.rs` | Client API | `/clients/*` |
| `sessions.rs` | Session API | `/sessions/*` |

### Service Layer

**Location:** `src/service/`

Business logic:

```rust
// src/service/token.rs
pub struct TokenService {
    cache: Arc<Cache>,
    db: Arc<DbPool>,
}

impl TokenService {
    pub async fn generate_token(
        &self,
        user: &User,
        client: &Client,
    ) -> Result<TokenPair, Error> {
        // Generate access token
        let access_token = self.create_access_token(user, client)?;
        
        // Generate refresh token
        let refresh_token = self.create_refresh_token(user, client)?;
        
        // Store in cache
        self.cache.set(&access_token).await?;
        
        Ok(TokenPair {
            access_token,
            refresh_token,
        })
    }
}
```

### Data Layer

**Location:** `src/data/`

Database models and queries:

```rust
// src/data/user.rs
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct User {
    pub id: String,
    pub email: String,
    pub password_hash: Option<String>,
    pub mfa_enabled: bool,
    pub created_at: DateTime<Utc>,
}

impl User {
    pub async fn find_by_email(
        db: &DbPool,
        email: &str,
    ) -> Result<Option<User>, Error> {
        sqlx::query_as(
            "SELECT * FROM users WHERE email = $1",
        )
        .bind(email)
        .fetch_optional(db)
        .await
    }
}
```

### JWT Layer

**Location:** `src/jwt/`

JWT token handling:

```rust
// src/jwt/mod.rs
pub struct JwtHandler {
    signing_key: Ed25519KeyPair,
    validation: Validation,
}

impl JwtHandler {
    pub fn sign(&self, claims: &Claims) -> Result<String, Error> {
        let token = encode(
            &Header::new(Algorithm::EdDSA),
            claims,
            &self.signing_key,
        )?;
        Ok(token)
    }
    
    pub fn verify(&self, token: &str) -> Result<Claims, Error> {
        let validation = self.validation.clone();
        let token_data = decode(token, &self.decoding_key, &validation)?;
        Ok(token_data.claims)
    }
}
```

## Request Flow

### OIDC Authorization Flow

```mermaid
sequenceDiagram
    participant Client
    participant Rauthy
    participant User
    participant DB

    Client->>Rauthy: GET /authorize?client_id=...&redirect_uri=...
    Rauthy->>DB: Validate client
    DB-->>Rauthy: Client config
    
    alt Not authenticated
        Rauthy->>User: Redirect to login
        User->>Rauthy: POST /login (credentials)
        Rauthy->>DB: Verify user
        DB-->>Rauthy: User data
        Rauthy->>Rauthy: Create session
    end
    
    Rauthy->>User: Consent screen (if needed)
    User->>Rauthy: Approve
    
    Rauthy->>Rauthy: Generate authorization code
    Rauthy->>DB: Store code
    Rauthy->>Client: Redirect with code
    
    Client->>Rauthy: POST /token (code + PKCE)
    Rauthy->>DB: Verify code
    Rauthy->>Rauthy: Generate tokens
    Rauthy->>Client: Access token + ID token
```

## Caching Architecture

```mermaid
flowchart TB
    subgraph Request["Request"]
        REQ[Incoming Request]
    end

    subgraph CacheCheck["Cache Check"]
        MEM[Memory Cache]
        DISK[Disk Cache]
    end

    subgraph DBAccess["Database"]
        HIQLITE[Hiqlite]
    end

    REQ --> MEM
    MEM -->|Hit| Return
    MEM -->|Miss| DISK
    DISK -->|Hit| MEM
    DISK -->|Miss| HIQLITE
    HIQLITE --> DISK
    DISK --> MEM
    MEM --> Return[Return Data]
```

**Cache levels:**
1. **Memory cache** — Fastest, per-request
2. **Distributed cache** — Hiqlite cache (HA mode)
3. **Database** — Persistent storage

## High Availability

### HA Mode with Hiqlite

```mermaid
flowchart TB
    subgraph Node1["Node 1"]
        APP1[rauthy]
        DB1[Hiqlite]
    end

    subgraph Node2["Node 2"]
        APP2[rauthy]
        DB2[Hiqlite]
    end

    subgraph Node3["Node 3"]
        APP3[rauthy]
        DB3[Hiqlite]
    end

    Node1 <-->|Raft| Node2
    Node2 <-->|Raft| Node3
    Node3 <-->|Raft| Node1
```

**Aha:** Hiqlite uses Raft consensus for distributed state. Each node has full data.

### HA Mode with Postgres

```mermaid
flowchart TB
    subgraph Nodes["rauthy Nodes"]
        N1[Node 1]
        N2[Node 2]
        N3[Node 3]
    end

    subgraph DB["PostgreSQL"]
        PRIMARY[Primary]
        REPLICA[Replica]
    end

    N1 --> PRIMARY
    N2 --> PRIMARY
    N3 --> PRIMARY
    PRIMARY --> REPLICA
```

## Next Steps

Continue to [Authentication →](02-authentication.html) for auth flows.
