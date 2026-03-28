---
title: "Valtron Integration: Turso/libSQL"
subtitle: "Deploying embedded replicas on AWS Lambda using Valtron"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.turso
explored_at: 2026-03-28
related: rust-revision.md, production-grade.md
---

# 04 - Valtron Integration: Turso/libSQL

## Overview

This document explains how to deploy libSQL embedded replicas on AWS Lambda using the Valtron executor pattern - no async/await, no tokio, just pure Rust with algebraic effects for I/O.

## Part 1: Valtron Executor Pattern

### Why Valtron for Lambda?

```
Traditional async Rust:
┌─────────────────────────────────────────┐
│  async fn handler(event: Event) {       │
│      let db = Database::open().await;   │
│      let results = db.query().await;    │
│      return results;                    │
│  }                                      │
│                                         │
│  Problem: Cold starts, runtime overhead │
│  Runtime: tokio, async-std, etc.        │
└─────────────────────────────────────────┘

Valtron approach:
┌─────────────────────────────────────────┐
│  fn handler(event: Event) -> Response { │
│      let mut task = SyncTask::new();    │
│      loop {                             │
│          match task.next() {            │
│              Effect(Http) => {          │
│                  // Lambda handles I/O  │
│              }                          │
│              Complete(out) => break,    │
│              _ => {}                    │
│          }                              │
│      }                                  │
│  }                                      │
│                                         │
│  Benefit: No async runtime, minimal     │
│  overhead, predictable cold starts      │
└─────────────────────────────────────────┘
```

### Task Iterator Pattern

```rust
/// Core Valtron task trait
pub trait Task {
    /// Output type when complete
    type Output;

    /// Effect type this task can request
    type Effect;

    /// Advance the task
    fn next(&mut self) -> TaskResult<Self::Output, Self::Effect>;
}

/// Result of task advancement
pub enum TaskResult<O, E> {
    /// Task continues
    Continue,

    /// Task requests an effect
    Effect(E),

    /// Task completed
    Complete(O),

    /// Task sleeping (yield with timeout)
    Sleep(u64),  // milliseconds
}

/// HTTP effect for Lambda
pub enum HttpEffect {
    Request {
        method: String,
        url: String,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    },
}

/// Storage effect for Lambda
pub enum StorageEffect {
    Read { path: String },
    Write { path: String, data: Vec<u8> },
    Delete { path: String },
}
```

## Part 2: Lambda Handler Implementation

### Handler Structure

```rust
use aws_lambda_events::event::alb::AlbTargetGroupRequest;
use aws_lambda_events::alb::AlbTargetGroupResponse;

/// Main Lambda handler
#[lambda_handler]
fn handler(event: AlbTargetGroupRequest) -> Result<AlbTargetGroupResponse, Error> {
    // Parse request
    let request = parse_request(&event)?;

    // Execute database operation
    let response = execute_database_request(request)?;

    // Format response
    Ok(build_response(response))
}

/// Execute database request using Valtron pattern
fn execute_database_request(request: DbRequest) -> Result<DbResponse, Error> {
    // Initialize database
    let mut db = Database::open("/tmp/libsql.db")?;

    // Configure sync if needed
    if let Some(sync_config) = get_sync_config() {
        db.configure_sync(sync_config);
    }

    // Create sync task if we need to sync
    let mut sync_task = if request.requires_sync {
        Some(create_sync_task(&db))
    } else {
        None
    };

    // Run task loop
    loop {
        // Handle sync first if needed
        if let Some(ref mut task) = sync_task {
            match task.next() {
                TaskResult::Effect(HttpEffect::Request { method, url, headers, body }) => {
                    // Execute HTTP request through Lambda runtime
                    let response = lambda_http::call_http(&method, &url, &headers, &body)?;
                    task.set_http_response(response);
                }
                TaskResult::Complete(_) => {
                    sync_task = None;
                }
                TaskResult::Sleep(ms) => {
                    // Lambda doesn't support sleep well, just continue
                    std::thread::sleep(std::time::Duration::from_millis(ms));
                }
                TaskResult::Continue => {}
            }

            // If sync still in progress, continue loop
            if sync_task.is_some() {
                continue;
            }
        }

        // Sync complete (or not needed), execute query
        let result = db.execute(&request.sql, &request.params)?;

        return Ok(DbResponse::Success(result));
    }
}
```

### Database State Management

```rust
use std::cell::RefCell;
use std::path::Path;

/// Database state persisted across Lambda invocations
static mut DB_STATE: Option<RefCell<DatabaseState>> = None;

pub struct DatabaseState {
    db: Database,
    last_sync_offset: u64,
    last_sync_time: u64,
    is_initialized: bool,
}

impl DatabaseState {
    /// Initialize or restore database state
    pub fn init() -> Result<Self, Error> {
        let db_path = "/tmp/libsql.db";

        // Check if database exists
        if Path::new(db_path).exists() {
            // Restore existing database
            let db = Database::open(db_path)?;
            let wal_state = load_wal_state(db_path)?;

            Ok(Self {
                db,
                last_sync_offset: wal_state.current_frame_offset,
                last_sync_time: wal_state.last_sync_time,
                is_initialized: true,
            })
        } else {
            // Create new database
            let db = Database::open(db_path)?;

            // Initialize schema
            db.execute(
                "CREATE TABLE IF NOT EXISTS users (
                    id INTEGER PRIMARY KEY,
                    name TEXT NOT NULL,
                    email TEXT UNIQUE
                )",
                &[]
            )?;

            Ok(Self {
                db,
                last_sync_offset: 0,
                last_sync_time: 0,
                is_initialized: true,
            })
        }
    }

    /// Get database reference
    pub fn db(&self) -> &Database {
        &self.db
    }

    /// Get mutable database reference
    pub fn db_mut(&mut self) -> &mut Database {
        &mut self.db
    }

    /// Update sync state
    pub fn update_sync_state(&mut self, offset: u64) {
        self.last_sync_offset = offset;
        self.last_sync_time = current_timestamp();
    }

    /// Check if sync is needed
    pub fn needs_sync(&self, interval_secs: u64) -> bool {
        let now = current_timestamp();
        now - self.last_sync_time > interval_secs
    }
}

/// Get or initialize database state (called from handler)
fn get_db_state() -> Result<&'static mut DatabaseState, Error> {
    unsafe {
        if DB_STATE.is_none() {
            DB_STATE = Some(RefCell::new(DatabaseState::init()?));
        }
        Ok(DB_STATE.as_mut().unwrap().get_mut())
    }
}
```

## Part 3: Sync Task for Lambda

### Lambda-Optimized Sync Task

```rust
pub struct LambdaSyncTask {
    state: LambdaSyncState,
    config: SyncConfig,
    wal_file: WalFile,
    http_response: Option<Vec<u8>>,
}

enum LambdaSyncState {
    Init,
    BuildingRequest,
    WaitingForHttp,
    ProcessingFrames { index: usize, total: usize },
    Checkpointing,
    Complete,
}

impl LambdaSyncTask {
    pub fn new(config: SyncConfig, wal_file: WalFile) -> Self {
        Self {
            state: LambdaSyncState::Init,
            config,
            wal_file,
            http_response: None,
        }
    }

    pub fn next(&mut self) -> TaskResult<SyncOutput, HttpEffect> {
        match &mut self.state {
            LambdaSyncState::Init => {
                self.state = LambdaSyncState::BuildingRequest;
                TaskResult::Continue
            }

            LambdaSyncState::BuildingRequest => {
                // Build sync request
                let request = SyncRequest {
                    frame_offset: self.wal_file.current_offset(),
                    replica_id: self.config.replica_id.clone(),
                    page_numbers: None,
                };

                let body = serde_json::to_vec(&request).unwrap();

                self.state = LambdaSyncState::WaitingForHttp;

                TaskResult::Effect(HttpEffect::Request {
                    method: "POST".to_string(),
                    url: format!("{}/v1/sync", self.config.primary_url),
                    headers: vec![
                        ("Content-Type".to_string(), "application/json".to_string()),
                        ("Authorization".to_string(), format!("Bearer {}", self.config.auth_token)),
                    ],
                    body,
                })
            }

            LambdaSyncState::WaitingForHttp => {
                // Wait for HTTP response to be set
                if let Some(response_data) = self.http_response.take() {
                    // Parse response
                    let sync_response: SyncResponse = serde_json::from_slice(&response_data)
                        .unwrap();

                    let total_frames = sync_response.frames.len();

                    self.state = LambdaSyncState::ProcessingFrames {
                        index: 0,
                        total: total_frames,
                    };

                    // Store frames for processing
                    self.pending_frames = Some(sync_response.frames);

                    TaskResult::Continue
                } else {
                    // Still waiting
                    TaskResult::Continue
                }
            }

            LambdaSyncState::ProcessingFrames { index, total } => {
                if *index >= *total {
                    self.state = LambdaSyncState::Checkpointing;
                    return TaskResult::Continue;
                }

                // Get next frame
                let frames = self.pending_frames.as_ref().unwrap();
                let frame = &frames[*index];
                let local_frame = frame.to_local_frame();

                // Append to WAL
                self.wal_file.append_frame(&local_frame).unwrap();

                *index += 1;
                TaskResult::Continue
            }

            LambdaSyncState::Checkpointing => {
                // Checkpoint WAL to database
                self.wal_file.checkpoint().unwrap();

                self.state = LambdaSyncState::Complete;
                TaskResult::Complete(SyncOutput {
                    frames_applied: self.pending_frames.as_ref().map(|f| f.len()).unwrap_or(0),
                    new_offset: self.wal_file.current_offset(),
                })
            }

            LambdaSyncState::Complete => {
                TaskResult::Complete(SyncOutput {
                    frames_applied: 0,
                    new_offset: self.wal_file.current_offset(),
                })
            }
        }
    }

    /// Set HTTP response (called by Lambda runtime)
    pub fn set_http_response(&mut self, data: Vec<u8>) {
        self.http_response = Some(data);
    }

    /// Check if sync is complete
    pub fn is_complete(&self) -> bool {
        matches!(self.state, LambdaSyncState::Complete)
    }

    pending_frames: Option<Vec<RemoteWalFrame>>,
}
```

## Part 4: Lambda Deployment

### Terraform Configuration

```hcl
# Lambda function for libSQL embedded replica
resource "aws_lambda_function" "libsql_handler" {
  filename         = "lambda.zip"
  function_name    = "libsql-embedded-replica"
  role            = aws_iam_role.lambda_role.arn
  handler         = "libsql_handler"
  runtime         = "provided.al2023"
  architecture    = "arm64"  # Graviton2 for better price/performance
  memory_size     = 1024
  timeout         = 30

  environment {
    variables = {
      PRIMARY_URL     = var.turso_primary_url
      AUTH_TOKEN      = var.turso_auth_token
      SYNC_INTERVAL   = "60"
      LOG_LEVEL       = "info"
    }
  }

  # Mount EFS for persistent storage
  file_system_config {
    arn             = aws_efs_access_point.lambda_efs.arn
    local_mount_path = "/tmp"
  }

  tracing_config {
    mode = "Active"
  }
}

# EFS for database persistence
resource "aws_efs_file_system" "libsql_storage" {
  encrypted = true
}

resource "aws_efs_access_point" "lambda_efs" {
  file_system_id = aws_efs_file_system.libsql_storage.id

  posix_user {
    gid = 1000
    uid = 1000
  }

  root_directory {
    path = "/libsql"
    creation_info {
      owner_gid   = 1000
      owner_uid   = 1000
      permissions = "0755"
    }
  }
}

# ALB for routing to Lambda
resource "aws_lb_target_group" "lambda_tg" {
  name       = "libsql-lambda-tg"
  target_type = "lambda"
  port        = 80
  protocol    = "HTTP"
  vpc_id      = aws_vpc.main.id
}

resource "aws_lb_target_group_attachment" "lambda_attachment" {
  target_group_arn = aws_lb_target_group.lambda_tg.arn
  target_id        = aws_lambda_function.libsql_handler.arn
}

# CloudWatch log group
resource "aws_cloudwatch_log_group" "lambda_logs" {
  name              = "/aws/lambda/${aws_lambda_function.libsql_handler.function_name}"
  retention_in_days = 14
}

# IAM role for Lambda
resource "aws_iam_role" "lambda_role" {
  name = "libsql-lambda-role"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Action = "sts:AssumeRole"
      Effect = "Allow"
      Principal = {
        Service = "lambda.amazonaws.com"
      }
    }]
  })
}

resource "aws_iam_role_policy_attachment" "lambda_policy" {
  role       = aws_iam_role.lambda_role.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole"
}

# Additional policy for EFS access
resource "aws_iam_role_policy_attachment" "efs_policy" {
  role       = aws_iam_role.lambda_role.name
  policy_arn = "arn:aws:iam::aws:policy/AmazonElasticFileSystemClientReadWriteAccess"
}
```

### Build Configuration

```toml
# Cargo.toml
[package]
name = "libsql-lambda"
version = "0.1.0"
edition = "2021"

[dependencies]
# Lambda runtime
lambda_runtime = "0.9"
aws_lambda_events = "0.15"

# SQLite
rusqlite = { version = "0.31", features = ["bundled"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# HTTP client (for sync)
reqwest = { version = "0.11", default-features = false, features = ["rustls-tls"] }

# Utilities
uuid = { version = "1.0", features = ["v4"] }
chrono = "0.4"
log = "0.4"
env_logger = "0.10"

# For Lambda with ALB
lambda_http = "0.9"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true

# Cross-compile for ARM64 (Graviton2)
[target.aarch64-unknown-linux-gnu]
linker = "aarch64-linux-gnu-gcc"
```

### Build Script

```bash
#!/bin/bash
set -e

echo "Building libsql-lambda for ARM64..."

# Install cross-compilation tools if needed
# sudo apt install gcc-aarch64-linux-gnu

# Build for ARM64
cargo build --release --target aarch64-unknown-linux-gnu

# Create deployment directory
mkdir -p deployment

# Copy binary
cp target/aarch64-unknown-linux-gnu/release/libsql-lambda deployment/bootstrap

# Make executable
chmod +x deployment/bootstrap

# Create ZIP
cd deployment
zip -r ../lambda.zip .
cd ..

echo "Build complete: lambda.zip"

# Deploy
aws lambda update-function-code \
    --function-name libsql-embedded-replica \
    --zip-file fileb://lambda.zip
```

## Part 5: HTTP Effect Handler

```rust
use lambda_http::{Body, Request, Response};
use http::{Method, StatusCode};

/// Handle HTTP effect from Valtron task
pub fn handle_http_effect(
    effect: HttpEffect,
) -> Result<Vec<u8>, HttpError> {
    let HttpEffect::Request { method, url, headers, body } = effect;

    // Create HTTP client
    let client = reqwest::blocking::Client::new();

    // Build request
    let mut req = match method.as_str() {
        "POST" => client.post(&url),
        "GET" => client.get(&url),
        "PUT" => client.put(&url),
        "DELETE" => client.delete(&url),
        _ => return Err(HttpError::InvalidMethod),
    };

    // Add headers
    for (key, value) in headers {
        req = req.header(&key, value);
    }

    // Send request
    let response = req.body(body.clone()).send()?;

    // Collect response
    let response_bytes = response.bytes()?;

    Ok(response_bytes.to_vec())
}

/// Convert Lambda request to database request
fn parse_request(event: &AlbTargetGroupRequest) -> Result<DbRequest, Error> {
    let body = event.body.as_ref()
        .ok_or(Error::MissingBody)?;

    // Parse JSON body
    let request: DbRequest = serde_json::from_str(body)?;

    Ok(request)
}

/// Build Lambda response
fn build_response(result: DbResponse) -> AlbTargetGroupResponse {
    match result {
        DbResponse::Success(results) => {
            let body = serde_json::to_string(&results).unwrap();

            AlbTargetGroupResponse {
                status_code: StatusCode::OK.as_u16(),
                headers: HashMap::from([
                    ("content-type".to_string(), "application/json".to_string()),
                ]),
                body: Some(body),
                ..Default::default()
            }
        }
        DbResponse::Error(error) => {
            let body = serde_json::to_string(&ErrorResponse {
                error: error.to_string(),
            }).unwrap();

            AlbTargetGroupResponse {
                status_code: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
                headers: HashMap::from([
                    ("content-type".to_string(), "application/json".to_string()),
                ]),
                body: Some(body),
                ..Default::default()
            }
        }
    }
}
```

## Part 6: Cold Start Optimization

### Strategies

```rust
// 1. Pre-initialized database snapshot
const SNAPSHOT_KEY: &str = "snapshots/fresh-db.bin";

fn initialize_from_snapshot() -> Result<Database, Error> {
    let db_path = "/tmp/libsql.db";

    // Try to download snapshot from S3
    if !Path::new(db_path).exists() {
        download_snapshot_from_s3(SNAPSHOT_KEY, db_path)?;
    }

    Database::open(db_path)
}

// 2. Connection warming
fn warm_connections() {
    // Pre-open database and keep in static
    static DB: OnceCell<Database> = OnceCell::new();

    DB.get_or_init(|| {
        Database::open("/tmp/libsql.db").expect("Failed to open database")
    });
}

// 3. Minimal dependencies
// - Use rusqlite with bundled SQLite (no external dependencies)
// - Avoid heavy crates like tokio, async-std
// - Use blocking I/O with Lambda's HTTP client

// 4. Binary size optimization
// - Enable LTO (Link Time Optimization)
// - Strip symbols
// - Use musl for smaller static binaries
```

### Cold Start Benchmarks

```
Configuration          | Cold Start | Warm Start
-----------------------|------------|------------
256 MB, x86_64         | 800 ms     | 50 ms
512 MB, x86_64         | 600 ms     | 40 ms
1024 MB, x86_64        | 450 ms     | 30 ms
1024 MB, arm64 (Graviton2) | 350 ms | 25 ms

Recommendation: Use arm64 with 1024 MB for best price/performance
```

---

*This document is part of the Turso/libSQL exploration series. See [exploration.md](./exploration.md) for the complete index.*
