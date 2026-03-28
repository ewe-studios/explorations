# SpacetimeDB: Valtron Integration for Serverless Lambda Deployment

## Overview

This document covers deploying SpacetimeDB to AWS Lambda using the valtron executor pattern:
- No async/await, no tokio
- Lambda invocation model
- Request/response handling
- State persistence with DynamoDB/S3
- Connection management

---

## 1. Valtron Executor Pattern

### 1.1 TaskIterator for Lambda

```rust
/// Valtron TaskIterator for Lambda execution
pub trait TaskIterator {
    type Ready;
    type Pending;
    type Spawner;
    type Error;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner, Self::Error>>;
}

pub enum TaskStatus<Ready, Pending, Spawner, Error> {
    Ready(Result<Ready, Error>),
    Pending(Pending),
    Spawned(Spawner),
    Done,
}

/// Lambda executor
pub struct LambdaExecutor {
    tasks: Vec<Box<dyn TaskIterator<Ready = LambdaResponse, Pending = (), Spawner = (), Error = LambdaError>>>,
}

impl LambdaExecutor {
    pub fn new() -> Self {
        Self { tasks: Vec::new() }
    }

    pub fn add_task(&mut self, task: Box<dyn TaskIterator<Ready = LambdaResponse, Pending = (), Spawner = (), Error = LambdaError>>) {
        self.tasks.push(task);
    }

    /// Execute all tasks to completion
    pub fn execute_all(&mut self) -> Vec<Result<LambdaResponse, LambdaError>> {
        let mut results = Vec::new();

        while !self.tasks.is_empty() {
            let mut completed = Vec::new();

            for (i, task) in self.tasks.iter_mut().enumerate() {
                match task.next() {
                    Some(TaskStatus::Ready(result)) => {
                        completed.push(i);
                        results.push(result);
                    }
                    Some(TaskStatus::Done) => {
                        completed.push(i);
                    }
                    _ => {}
                }
            }

            // Remove completed tasks (in reverse order to maintain indices)
            for i in completed.into_iter().rev() {
                self.tasks.remove(i);
            }

            // Yield to Lambda runtime if needed
            if !self.tasks.is_empty() {
                std::thread::yield_now();
            }
        }

        results
    }
}
```

### 1.2 Lambda Request Handler

```rust
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use serde::{Deserialize, Serialize};

/// Lambda request
#[derive(Debug, Deserialize)]
pub struct LambdaRequest {
    #[serde(rename = "resource")]
    pub resource: String,

    #[serde(rename = "path")]
    pub path: String,

    #[serde(rename = "httpMethod")]
    pub http_method: String,

    #[serde(rename = "headers")]
    pub headers: HashMap<String, String>,

    #[serde(rename = "body")]
    pub body: Option<String>,

    #[serde(rename = "queryStringParameters")]
    pub query_params: Option<HashMap<String, String>>,
}

/// Lambda response
#[derive(Debug, Serialize)]
pub struct LambdaResponse {
    #[serde(rename = "statusCode")]
    pub status_code: u16,

    #[serde(rename = "headers")]
    pub headers: HashMap<String, String>,

    #[serde(rename = "body")]
    pub body: String,
}

/// Database request
#[derive(Debug, Deserialize)]
#[serde(tag = "action")]
pub enum DatabaseRequest {
    #[serde(rename = "query")]
    Query { sql: String },

    #[serde(rename = "insert")]
    Insert { table: String, data: serde_json::Value },

    #[serde(rename = "update")]
    Update { table: String, id: u64, data: serde_json::Value },

    #[serde(rename = "delete")]
    Delete { table: String, id: u64 },
}

/// Main Lambda handler
pub async fn handler(event: LambdaEvent<LambdaRequest>) -> Result<LambdaResponse, Error> {
    let request = event.payload;

    // Parse database operation from request body
    let db_request: DatabaseRequest = if let Some(body) = &request.body {
        serde_json::from_str(body)?
    } else {
        return Ok(LambdaResponse {
            status_code: 400,
            headers: HashMap::new(),
            body: "Missing request body".into(),
        });
    };

    // Execute database operation using valtron pattern
    let response = execute_database_operation(db_request)?;

    Ok(response)
}

/// Execute database operation (no async!)
fn execute_database_operation(request: DatabaseRequest) -> Result<LambdaResponse, LambdaError> {
    // Load database state from S3/DynamoDB
    let mut db = Database::load_from_storage()?;

    // Execute operation
    let result = match request {
        DatabaseRequest::Query { sql } => {
            let rows = db.query(&sql)?;
            ResponseBody::Query { rows }
        }

        DatabaseRequest::Insert { table, data } => {
            let id = db.insert(&table, data)?;
            ResponseBody::Insert { id }
        }

        DatabaseRequest::Update { table, id, data } => {
            db.update(&table, id, data)?;
            ResponseBody::Update { success: true }
        }

        DatabaseRequest::Delete { table, id } => {
            db.delete(&table, id)?;
            ResponseBody::Delete { success: true }
        }
    };

    // Save database state back to storage
    db.save_to_storage()?;

    // Build Lambda response
    Ok(LambdaResponse {
        status_code: 200,
        headers: {
            let mut h = HashMap::new();
            h.insert("Content-Type".into(), "application/json".into());
            h
        },
        body: serde_json::to_string(&result)?,
    })
}

#[derive(Serialize)]
#[serde(untagged)]
enum ResponseBody {
    Query { rows: Vec<serde_json::Value> },
    Insert { id: u64 },
    Update { success: bool },
    Delete { success: bool },
}

#[derive(Debug, thiserror::Error)]
pub enum LambdaError {
    #[error("Database error: {0}")]
    Database(#[from] DatabaseError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Storage error: {0}")]
    Storage(String),
}
```

---

## 2. State Persistence

### 2.1 S3 for Database Snapshots

```rust
use aws_sdk_s3::{Client as S3Client, config::Config};
use aws_config::{load_from_env, BehaviorVersion};

/// S3 storage for database snapshots
pub struct S3Storage {
    client: S3Client,
    bucket: String,
    key_prefix: String,
}

impl S3Storage {
    pub async fn new(bucket: String, key_prefix: String) -> Result<Self> {
        let config = load_from_env().await;
        let client = S3Client::new(&config);

        Ok(Self {
            client,
            bucket,
            key_prefix,
        })
    }

    /// Save database snapshot to S3
    pub fn save_snapshot(&self, snapshot: &DatabaseSnapshot, version: u64) -> Result<()> {
        // Serialize snapshot
        let bytes = bincode::serialize(snapshot)?;

        // Compress
        let mut encoder = flate2::write::GzEncoder::new(
            Vec::new(),
            flate2::Compression::default(),
        );
        encoder.write_all(&bytes)?;
        let compressed = encoder.finish()?;

        // Upload to S3 (blocking call for Lambda)
        let key = format!("{}/snapshot_v{}.bin.gz", self.key_prefix, version);

        // Use blocking runtime for S3 upload
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async {
            self.client
                .put_object()
                .bucket(&self.bucket)
                .key(&key)
                .content_type("application/octet-stream")
                .body(compressed.into())
                .send()
                .await
        })?;

        Ok(())
    }

    /// Load latest snapshot from S3
    pub fn load_snapshot(&self) -> Result<DatabaseSnapshot> {
        // List objects to find latest version
        let objects = self.list_objects(&self.key_prefix)?;
        let latest = objects.into_iter()
            .filter(|k| k.contains("snapshot_") && k.ends_with(".bin.gz"))
            .max()
            .ok_or("No snapshots found")?;

        // Download from S3
        let response = self.get_object(&latest)?;

        // Decompress
        let mut decoder = flate2::read::GzDecoder::new(response.as_slice());
        let mut bytes = Vec::new();
        decoder.read_to_end(&mut bytes)?;

        // Deserialize
        let snapshot = bincode::deserialize(&bytes)?;

        Ok(snapshot)
    }
}
```

### 2.2 DynamoDB for Metadata

```rust
use aws_sdk_dynamodb::{Client as DynamoDbClient, types::AttributeValue};

/// DynamoDB for database metadata
pub struct DynamoDbStorage {
    client: DynamoDbClient,
    table_name: String,
}

impl DynamoDbStorage {
    pub fn new(table_name: String) -> Result<Self> {
        // Note: In Lambda, we'd use the blocking DynamoDB client
        // or wrap the async client in a blocking executor
        Ok(Self {
            table_name,
            client: DynamoDbClient::uninit(),  // Placeholder
        })
    }

    /// Get current database version
    pub fn get_version(&self, db_id: &str) -> Result<u64> {
        // DynamoDB get_item (blocking)
        let response = self.get_item(db_id)?;

        let version = response
            .item()
            .and_then(|item| item.get("version"))
            .and_then(|v| v.as_n())
            .and_then(|n| n.parse::<u64>().ok())
            .unwrap_or(0);

        Ok(version)
    }

    /// Update database version
    pub fn update_version(&self, db_id: &str, version: u64) -> Result<()> {
        // DynamoDB update_item (blocking)
        self.update_item(db_id, version)?;
        Ok(())
    }

    /// Store commitlog entries
    pub fn append_commitlog(&self, db_id: &str, entries: Vec<CommitlogEntry>) -> Result<()> {
        for entry in entries {
            self.put_commitlog_entry(db_id, entry)?;
        }
        Ok(())
    }
}
```

### 2.3 Hybrid Storage Strategy

```rust
/// Combined S3 + DynamoDB storage
pub struct HybridStorage {
    s3: S3Storage,
    dynamodb: DynamoDbStorage,
}

impl HybridStorage {
    pub fn new(bucket: String, table_name: String) -> Result<Self> {
        Ok(Self {
            s3: S3Storage::new(bucket, "snapshots".into())?,
            dynamodb: DynamoDbStorage::new(table_name)?,
        })
    }

    /// Save database state
    pub fn save(&self, db: &Database) -> Result<()> {
        // Get current version
        let mut version = self.get_version(db.id())?;
        version += 1;

        // Create snapshot
        let snapshot = db.create_snapshot();

        // Save to S3
        self.s3.save_snapshot(&snapshot, version)?;

        // Update version in DynamoDB
        self.dynamodb.update_version(db.id(), version)?;

        // Save commitlog entries to DynamoDB
        self.dynamodb.append_commitlog(db.id(), db.pending_commitlog())?;

        Ok(())
    }

    /// Load database state
    pub fn load(&self, db_id: &str) -> Result<Database> {
        // Get version
        let version = self.dynamodb.get_version(db_id)?;

        // Load snapshot from S3
        let snapshot = self.s3.load_snapshot()?;

        // Create database from snapshot
        let mut db = Database::from_snapshot(snapshot)?;

        // Replay commitlog from DynamoDB
        let commitlog = self.dynamodb.get_commitlog(db_id)?;
        for entry in commitlog {
            db.apply_commitlog_entry(entry)?;
        }

        Ok(db)
    }
}
```

---

## 3. Lambda Deployment

### 3.1 Lambda Function Package

```toml
# Cargo.toml
[package]
name = "spacetimedb-lambda"
version = "0.1.0"
edition = "2021"

[dependencies]
lambda_runtime = "0.10"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1", features = ["rt", "macros"] }
aws-config = "1.0"
aws-sdk-s3 = "1.0"
aws-sdk-dynamodb = "1.0"
bincode = "1.3"
flate2 = "1.0"
thiserror = "1.0"

[[bin]]
name = "bootstrap"
path = "src/main.rs"
```

### 3.2 Lambda Entry Point

```rust
// src/main.rs
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use spacetimedb_lambda::{handler, LambdaRequest, LambdaResponse};

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    // Run Lambda function
    run(service_fn(handler)).await?;

    Ok(())
}
```

### 3.3 SAM Template

```yaml
# template.yaml
AWSTemplateFormatVersion: '2010-09-09'
Transform: AWS::Serverless-2016-10-31

Resources:
  SpacetimeDBFunction:
    Type: AWS::Serverless::Function
    Properties:
      FunctionName: spacetimedb-lambda
      CodeUri: target/lambda/spacetimedb-lambda/
      Handler: bootstrap
      Runtime: provided.al2
      Architecture: x86_64
      MemorySize: 1024
      Timeout: 30
      Environment:
        Variables:
          RUST_LOG: info
          S3_BUCKET: !Ref DatabaseBucket
          DYNAMODB_TABLE: !Ref MetadataTable
      Policies:
        - S3CrudPolicy:
            BucketName: !Ref DatabaseBucket
        - DynamoDBCrudPolicy:
            TableName: !Ref MetadataTable
      Events:
        Api:
          Type: Api
          Properties:
            Path: /{proxy+}
            Method: ANY

  DatabaseBucket:
    Type: AWS::S3::Bucket
    Properties:
      BucketName: spacetimedb-database
      VersioningConfiguration:
        Status: Enabled
      LifecycleConfiguration:
        Rules:
          - Id: CleanupOldVersions
            Status: Enabled
            NoncurrentVersionExpiration:
              NoncurrentDays: 7

  MetadataTable:
    Type: AWS::DynamoDB::Table
    Properties:
      TableName: spacetimedb-metadata
      AttributeDefinitions:
        - AttributeName: db_id
          AttributeType: S
      KeySchema:
        - AttributeName: db_id
          KeyType: HASH
      BillingMode: PAY_PER_REQUEST
      StreamSpecification:
        StreamViewType: NEW_AND_OLD_IMAGES

Outputs:
  ApiEndpoint:
    Description: API Gateway endpoint URL
    Value: !Sub "https://${ServerlessRestApi}.execute-api.${AWS::Region}.amazonaws.com/Prod/"
```

### 3.4 Deployment Script

```bash
#!/bin/bash
# deploy.sh

set -e

# Build for Lambda
cargo build --release --target x86_64-unknown-linux-gnu

# Create Lambda package
mkdir -p target/lambda/spacetimedb-lambda
cp target/x86_64-unknown-linux-gnu/release/bootstrap target/lambda/spacetimedb-lambda/

# Deploy with SAM
sam build
sam deploy --stack-name spacetimedb-lambda --config-file samconfig.toml --resolve-s3

echo "Deployment complete!"
```

---

## 4. Performance Optimization for Lambda

### 4.1 Connection Pooling

```rust
use once_cell::sync::Lazy;
use parking_lot::Mutex;

/// Global connection pool (reused across Lambda invocations)
static CONNECTION_POOL: Lazy<Mutex<ConnectionPool>> = Lazy::new(|| {
    Mutex::new(ConnectionPool::new(10))
});

/// Initialize once, reuse across invocations
pub fn init_storage() -> Result<()> {
    let pool = CONNECTION_POOL.lock();
    // Initialize connections
    Ok(())
}

/// Get connection from pool
pub fn get_connection() -> Result<Connection> {
    CONNECTION_POOL.lock().acquire()
}

/// Return connection to pool
pub fn release_connection(conn: Connection) {
    CONNECTION_POOL.lock().release(conn);
}
```

### 4.2 Cold Start Optimization

```rust
use std::time::Instant;

/// Track initialization time
static mut INIT_TIME: Option<Instant> = None;

#[no_mangle]
pub extern "C" fn __libc_start_main(main: fn() -> i32) -> ! {
    unsafe {
        INIT_TIME = Some(Instant::now());
    }

    // Continue with normal startup
    extern "C" {
        fn __libc_start_main_real(main: fn() -> i32) -> !;
    }
    unsafe { __libc_start_main_real(main) }
}

/// Log initialization time
fn log_cold_start() {
    if let Some(init_time) = unsafe { INIT_TIME } {
        let elapsed = init_time.elapsed();
        eprintln!("Lambda cold start: {:.2?}ms", elapsed.as_secs_f64() * 1000.0);
    }
}
```

### 4.3 Memory Management

```rust
/// Memory-efficient query execution
pub struct MemoryEfficientQuery {
    /// Process rows in batches
    batch_size: usize,

    /// Stream results instead of collecting
    streaming: bool,
}

impl MemoryEfficientQuery {
    pub fn new(batch_size: usize) -> Self {
        Self {
            batch_size,
            streaming: true,
        }
    }

    pub fn execute_streaming<F>(&self, db: &Database, sql: &str, mut f: F) -> Result<()>
    where
        F: FnMut(&Row) -> Result<()>,
    {
        let mut row_count = 0;

        // Process in batches
        for batch in db.scan_batches(sql, self.batch_size) {
            for row in batch? {
                f(&row)?;
                row_count += 1;

                // Check Lambda memory limit
                if row_count % 1000 == 0 {
                    if self.memory_usage_exceeded() {
                        return Err("Memory limit exceeded".into());
                    }
                }
            }
        }

        Ok(())
    }

    fn memory_usage_exceeded(&self) -> bool {
        // Check against Lambda memory limit
        let limit = std::env::var("AWS_LAMBDA_FUNCTION_MEMORY_SIZE")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(128) * 1024 * 1024 / 8;  // Use 1/8th of allocated memory

        let usage = self.get_memory_usage();
        usage > limit
    }
}
```

---

## 5. Example: Serverless Database API

### 5.1 Complete Lambda Function

```rust
// src/lib.rs
use lambda_runtime::{Error, LambdaEvent};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct ApiRequest {
    pub resource: String,
    pub path: String,
    pub http_method: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
}

pub async fn handler(event: LambdaEvent<ApiRequest>) -> Result<ApiResponse, Error> {
    let request = event.payload;

    // Route based on path and method
    let response = match (request.http_method.as_str(), request.path.as_str()) {
        ("GET", "/tables") => list_tables(),
        ("GET", path) if path.starts_with("/tables/") => {
            let table = path.strip_prefix("/tables/").unwrap();
            get_table_data(table, &request)
        }
        ("POST", path) if path.starts_with("/tables/") => {
            let table = path.strip_prefix("/tables/").unwrap();
            insert_row(table, &request)
        }
        ("PUT", path) if path.starts_with("/tables/") => {
            let parts: Vec<&str> = path.split('/').collect();
            if parts.len() >= 4 {
                update_row(parts[2], parts[3].parse()?, &request)
            } else {
                bad_request("Invalid path")
            }
        }
        ("DELETE", path) if path.starts_with("/tables/") => {
            let parts: Vec<&str> = path.split('/').collect();
            if parts.len() >= 4 {
                delete_row(parts[2], parts[3].parse()?)
            } else {
                bad_request("Invalid path")
            }
        }
        _ => not_found(),
    };

    Ok(response)
}

fn list_tables() -> ApiResponse {
    let db = load_database();
    let tables = db.list_tables();

    json_response(200, &serde_json::json!({ "tables": tables }))
}

fn get_table_data(table: &str, request: &ApiRequest) -> ApiResponse {
    let db = load_database();

    // Get query params
    let query = request
        .headers
        .get("x-query")
        .map(|s| s.as_str())
        .unwrap_or("SELECT * FROM table");

    match db.query(query) {
        Ok(rows) => json_response(200, &serde_json::json!({ "rows": rows })),
        Err(e) => error_response(500, &e.to_string()),
    }
}

fn insert_row(table: &str, request: &ApiRequest) -> ApiResponse {
    let body = match &request.body {
        Some(b) => b,
        None => return bad_request("Missing body"),
    };

    let data: serde_json::Value = match serde_json::from_str(body) {
        Ok(d) => d,
        Err(e) => return bad_request(&e.to_string()),
    };

    let mut db = load_database();

    match db.insert(table, data) {
        Ok(id) => {
            db.save();
            json_response(201, &serde_json::json!({ "id": id }))
        }
        Err(e) => error_response(500, &e.to_string()),
    }
}

fn load_database() -> Database {
    Database::load_from_storage().expect("Failed to load database")
}

fn json_response<T: Serialize>(status: u16, data: &T) -> ApiResponse {
    let mut headers = HashMap::new();
    headers.insert("Content-Type".into(), "application/json".into());

    ApiResponse {
        status_code: status,
        headers,
        body: serde_json::to_string(data).unwrap(),
    }
}

fn bad_request(msg: &str) -> ApiResponse {
    error_response(400, msg)
}

fn not_found() -> ApiResponse {
    error_response(404, "Not found")
}

fn error_response(status: u16, msg: &str) -> ApiResponse {
    json_response(status, &serde_json::json!({ "error": msg }))
}
```

---

## Document History

| Date | Change |
|------|--------|
| 2026-03-27 | Initial Valtron integration guide created |

---

*This exploration is a living document. Revisit sections as concepts become clearer through implementation.*
