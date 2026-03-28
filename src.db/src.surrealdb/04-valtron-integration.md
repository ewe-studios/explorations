# SurrealDB: Valtron Integration for Lambda

## Overview

Deploy SurrealDB to AWS Lambda using valtron pattern (no async/await).

---

## 1. Lambda Handler

```rust
use lambda_runtime::{Error, LambdaEvent};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct ApiRequest {
    pub http_method: String,
    pub path: String,
    pub body: Option<String>,
}

#[derive(Serialize)]
pub struct ApiResponse {
    pub status_code: u16,
    pub body: String,
}

pub fn handler(event: LambdaEvent<ApiRequest>) -> Result<ApiResponse, Error> {
    let request = event.payload;

    // Load database (from S3/DynamoDB)
    let mut db = Database::load()?;

    // Execute query
    let response = match request.http_method.as_str() {
        "GET" => handle_get(&db, &request.path),
        "POST" => handle_post(&mut db, &request.path, &request.body),
        _ => not_found(),
    };

    // Save database
    db.save()?;

    Ok(response)
}
```

## 2. State Persistence

```rust
impl Database {
    pub fn load() -> Result<Self> {
        // Load from S3
        let snapshot = s3_get("bucket", "snapshot.bin")?;
        Self::from_snapshot(&snapshot)
    }

    pub fn save(&self) -> Result<()> {
        let snapshot = self.to_snapshot();
        s3_put("bucket", "snapshot.bin", &snapshot)
    }
}
```

---

## Document History

| Date | Change |
|------|--------|
| 2026-03-27 | Initial Valtron integration created |

---

*This exploration is a living document.*
