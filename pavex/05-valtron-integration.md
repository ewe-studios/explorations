---
title: "Valtron Integration: Lambda Deployment for Build Services"
subtitle: "Deploying Pavex-like build tools on AWS Lambda using Valtron (NO async/tokio)"
based_on: "foundation_core/src/valtron/ and pavex build architecture"
level: "Advanced - Lambda and Valtron expertise required"
---

# Valtron Integration: Lambda Deployment for Build Services

## Overview

This guide explains how to deploy Pavex-like build services on AWS Lambda using **Valtron** instead of tokio/async-runtime. The key insight: **build tools don't need async** - they need deterministic, sequential execution with clear state management.

**Why Valtron for Lambda?**

| Requirement | Tokio/Async | Valtron |
|-------------|-------------|---------|
| Cold start | Runtime init (~100ms) | Zero overhead |
| Lambda pause | Loses async state | State in struct |
| Determinism | Non-deterministic scheduling | Step-by-step execution |
| Memory | Runtime + tasks | Just your structs |
| Serialization | Complex | Simple struct fields |

---

## 1. Why Async Doesn't Fit Lambda

### 1.1 The Lambda Execution Model

```
┌─────────────────────────────────────────────────────────┐
│              AWS Lambda Lifecycle                        │
│                                                          │
│  1. INIT: Runtime starts, handler loaded                │
│  2. INVOKE: Handler called with event                   │
│  3. PROCESS: Your code runs                             │
│  4. RESPONSE: Return result                             │
│  5. FREEZE: Lambda pauses (state lost!)                 │
│  6. UNFREEZE: Lambda resumes (new context)              │
│                                                          │
│  Billable time: Steps 1-4 (milliseconds precision)      │
└─────────────────────────────────────────────────────────┘
```

### 1.2 Async Problems on Lambda

**Problem 1: Runtime initialization**

```rust
// Tokio runtime initialization adds latency
#[tokio::main]
async fn main() -> Result<(), Error> {
    // Runtime starts here (~50-100ms cold start overhead)
    handler(event).await
}
```

**Problem 2: Lambda freeze loses async state**

```rust
// This async task gets interrupted on Lambda freeze
async fn process_build(request: BuildRequest) -> BuildResult {
    // Step 1: Fetch rustdoc JSON (may be frozen mid-request)
    let docs = fetch_rustdoc(&request.crate_name).await?;

    // Step 2: Analyze (state lost if frozen)
    let analysis = analyze(&docs).await?;

    // Step 3: Generate (never reached if frozen)
    generate(&analysis)
}
```

**Problem 3: No meaningful concurrency**

Build steps are **sequential**:
```
rustdoc JSON -> analyze -> generate -> compile
```

You can't parallelize these steps meaningfully.

### 1.3 Valtron Solution

```rust
// Valtron: Explicit state machine
struct BuildTask {
    state: BuildState,
    request: BuildRequest,
    result: Option<BuildResult>,
}

enum BuildState {
    FetchingDocs { url: String },
    Analyzing { docs: RustdocJson },
    Generating { analysis: Analysis },
    Done,
}

impl TaskIterator for BuildTask {
    type Ready = BuildResult;
    type Pending = BuildState;
    type Spawner = NoSpawner;

    fn next_status(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        match std::mem::replace(&mut self.state, BuildState::Done) {
            BuildState::FetchingDocs { url } => {
                let docs = fetch_rustdoc_sync(&url);
                self.state = BuildState::Analyzing { docs };
                Some(TaskStatus::Pending(BuildState::Analyzing { docs }))
            }
            BuildState::Analyzing { docs } => {
                let analysis = analyze_sync(&docs);
                self.state = BuildState::Generating { analysis };
                Some(TaskStatus::Pending(BuildState::Generating { analysis }))
            }
            BuildState::Generating { analysis } => {
                let result = generate_sync(&analysis);
                self.result = Some(result.clone());
                Some(TaskStatus::Ready(result))
            }
            BuildState::Done => None,
        }
    }
}
```

---

## 2. Valtron Executor Patterns

### 2.1 Single-Threaded Executor for Lambda

```rust
// build_service/src/executor.rs
use valtron::{TaskIterator, TaskStatus, NoSpawner, execute};

pub struct BuildExecutor {
    seed: u64,
}

impl BuildExecutor {
    pub fn new() -> Self {
        Self {
            seed: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        }
    }

    pub fn execute_build(&self, request: BuildRequest) -> Result<BuildResult, BuildError> {
        // Initialize single-threaded executor
        valtron::single::initialize_pool(self.seed);

        // Create build task
        let task = BuildTask::new(request);

        // Execute to completion (deterministic)
        let results: Vec<BuildResult> = execute()
            .with_task(task)
            .schedule_iter(Duration::from_millis(10))?
            .collect();

        // Run until all tasks complete
        valtron::single::run_until_complete();

        results.into_iter().next().ok_or(BuildError::NoResult)
    }
}
```

### 2.2 TaskIterator for Build Steps

```rust
// build_service/src/tasks/build.rs
use valtron::{TaskIterator, TaskStatus, NoSpawner};

pub struct BuildTask {
    step: BuildStep,
    request: BuildRequest,
    intermediate: BuildIntermediate,
}

enum BuildStep {
    Init,
    FetchRustdoc,
    AnalyzeDependencies,
    BuildCallGraph,
    GenerateCode,
    Complete,
}

struct BuildIntermediate {
    rustdoc: Option<RustdocJson>,
    analysis: Option<Analysis>,
    call_graph: Option<CallGraph>,
    generated: Option<GeneratedCode>,
}

impl TaskIterator for BuildTask {
    type Ready = BuildResult;
    type Pending = BuildStep;
    type Spawner = NoSpawner;

    fn next_status(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        match std::mem::replace(&mut self.step, BuildStep::Complete) {
            BuildStep::Init => {
                self.step = BuildStep::FetchRustdoc;
                Some(TaskStatus::Pending(BuildStep::FetchRustdoc))
            }

            BuildStep::FetchRustdoc => {
                let rustdoc = fetch_rustdoc_sync(&self.request.crate_name);
                self.intermediate.rustdoc = Some(rustdoc.clone());
                self.step = BuildStep::AnalyzeDependencies;
                Some(TaskStatus::Pending(BuildStep::AnalyzeDependencies))
            }

            BuildStep::AnalyzeDependencies => {
                let analysis = analyze_deps_sync(self.intermediate.rustdoc.as_ref().unwrap());
                self.intermediate.analysis = Some(analysis.clone());
                self.step = BuildStep::BuildCallGraph;
                Some(TaskStatus::Pending(BuildStep::BuildCallGraph))
            }

            BuildStep::BuildCallGraph => {
                let call_graph = build_call_graph_sync(self.intermediate.analysis.as_ref().unwrap());
                self.intermediate.call_graph = Some(call_graph.clone());
                self.step = BuildStep::GenerateCode;
                Some(TaskStatus::Pending(BuildStep::GenerateCode))
            }

            BuildStep::GenerateCode => {
                let generated = generate_code_sync(self.intermediate.call_graph.as_ref().unwrap());
                self.intermediate.generated = Some(generated.clone());
                self.step = BuildStep::Complete;
                Some(TaskStatus::Ready(BuildResult {
                    code: generated,
                    diagnostics: vec![],
                }))
            }

            BuildStep::Complete => None,
        }
    }
}
```

### 2.3 DrivenRecvIterator for HTTP Requests

```rust
// build_service/src/tasks/http.rs
use valtron::{DrivenRecvIterator, DrivenStreamIterator, TaskStatus};

pub struct HttpFetchTask {
    url: String,
    state: HttpState,
}

enum HttpState {
    Waiting,
    Done(Result<Vec<u8>, HttpError>),
}

impl DrivenRecvIterator for HttpFetchTask {
    type Ready = Result<Vec<u8>, HttpError>;
    type Pending = ();

    fn next_status(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending>> {
        match &self.state {
            HttpState::Waiting => {
                // Synchronous HTTP fetch (use blocking client)
                let result = reqwest::blocking::get(&self.url)
                    .and_then(|r| r.bytes())
                    .map(|b| b.to_vec())
                    .map_err(|e| HttpError::Fetch(e.to_string()));

                self.state = HttpState::Done(result.clone());
                Some(TaskStatus::Ready(result))
            }
            HttpState::Done(_) => None,
        }
    }
}
```

---

## 3. Lambda Handler Implementation

### 3.1 Handler Structure

```rust
// build_service/src/lambda.rs
use aws_lambda_events::event::alb::AlbTargetGroupRequest;
use aws_lambda_events::alb::AlbTargetGroupResponse;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct BuildEvent {
    pub crate_name: String,
    pub version: String,
    pub features: Vec<String>,
}

#[derive(Serialize)]
pub struct BuildResponse {
    pub success: bool,
    pub sdk_url: Option<String>,
    pub error: Option<String>,
    pub duration_ms: u64,
}

/// Lambda handler - NO async!
#[inline(never)]
pub fn handler(event: AlbTargetGroupRequest) -> Result<AlbTargetGroupResponse, serde_json::Error> {
    let start = std::time::Instant::now();

    // Parse request
    let build_event: BuildEvent = serde_json::from_str(&event.body.unwrap_or_default())?;

    // Execute build synchronously
    let executor = BuildExecutor::new();
    let result = executor.execute_build(BuildRequest {
        crate_name: build_event.crate_name,
        version: build_event.version,
        features: build_event.features,
    });

    // Build response
    let (success, sdk_url, error) = match result {
        Ok(build_result) => {
            // Upload to S3
            let url = upload_to_s3(&build_result);
            (true, Some(url), None)
        }
        Err(e) => (false, None, Some(e.to_string())),
    };

    let response = BuildResponse {
        success,
        sdk_url,
        error,
        duration_ms: start.elapsed().as_millis() as u64,
    };

    Ok(AlbTargetGroupResponse {
        status_code: 200,
        headers: build_response_headers(),
        body: Some(serde_json::to_string(&response)?),
        ..Default::default()
    })
}

fn build_response_headers() -> HashMap<String, String> {
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    headers.insert("Access-Control-Allow-Origin".to_string(), "*".to_string());
    headers
}
```

### 3.2 main.rs Entry Point

```rust
// build_service/src/main.rs
mod lambda;
mod executor;
mod tasks;

use lambda_runtime::{run, service_fn, Error, LambdaEvent};

/// Use lambda_runtime's run, NOT tokio::main
#[tokio::main]
async fn main() -> Result<(), Error> {
    // Note: We still need tokio for lambda_runtime's event loop,
    // but OUR code is synchronous!

    let func = service_fn(|event: LambdaEvent<alb::AlbTargetGroupRequest>| async {
        // Our handler is synchronous - wrapped in async for lambda_runtime
        let response = lambda::handler(event.payload)?;
        Ok(response)
    });

    run(func).await?;
    Ok(())
}
```

### 3.3 Alternative: Pure Sync Handler

```rust
// For pure sync without lambda_runtime tokio dependency
// Use aws_lambda_events + custom runtime

use std::io::{self, Read, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read event from stdin (Lambda runtime protocol)
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    // Parse and handle
    let event: AlbTargetGroupRequest = serde_json::from_str(&input)?;
    let response = lambda::handler(event)?;

    // Write response to stdout
    let output = serde_json::to_string(&response)?;
    io::stdout().write_all(output.as_bytes())?;

    Ok(())
}
```

---

## 4. HTTP API Compatibility

### 4.1 API Gateway Integration

```yaml
# serverless.yml
service: pavex-build-service

provider:
  name: aws
  runtime: provided.al2
  region: us-east-1
  timeout: 30  # Max 30 seconds for build operations
  memorySize: 1024

functions:
  build:
    handler: bootstrap
    events:
      - http:
          path: /build
          method: POST
          cors: true
      - http:
          path: /status/{buildId}
          method: GET

plugins:
  - serverless-rust
```

### 4.2 Request/Response Types

```rust
// build_service/src/api.rs
use serde::{Deserialize, Serialize};

/// POST /build request
#[derive(Deserialize, Debug)]
pub struct BuildRequest {
    pub crate_name: String,
    pub version: String,
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default)]
    pub git_url: Option<String>,
}

/// POST /build response
#[derive(Serialize, Debug)]
pub struct BuildResponse {
    pub build_id: String,
    pub status: BuildStatus,
    pub estimated_duration_ms: u64,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum BuildStatus {
    Queued,
    Processing,
    Complete,
    Failed,
}

/// GET /status/{buildId} response
#[derive(Serialize, Debug)]
pub struct StatusResponse {
    pub build_id: String,
    pub status: BuildStatus,
    pub progress_percent: u8,
    pub sdk_url: Option<String>,
    pub error: Option<String>,
    pub logs: Vec<String>,
}

/// Internal task state
#[derive(Clone, Debug)]
pub struct BuildTaskState {
    pub build_id: String,
    pub request: BuildRequest,
    pub current_step: BuildStep,
    pub progress_percent: u8,
    pub result: Option<BuildResult>,
    pub error: Option<String>,
}

#[derive(Clone, Debug)]
pub enum BuildStep {
    Queued,
    FetchingSource,
    GeneratingRustdoc,
    Analyzing,
    GeneratingCode,
    Packaging,
    Complete,
}
```

---

## 5. S3 Storage Integration

### 5.1 Upload Generated SDK

```rust
// build_service/src/storage.rs
use aws_sdk_s3::{Client as S3Client, config::Credentials};

pub struct S3Storage {
    client: S3Client,
    bucket: String,
}

impl S3Storage {
    pub fn new(bucket: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // Use blocking S3 client
        let config = aws_config::from_env().load_sync();
        let client = S3Client::new(&config);

        Ok(Self {
            client,
            bucket: bucket.to_string(),
        })
    }

    pub fn upload_sdk_sync(
        &self,
        build_id: &str,
        sdk_bytes: &[u8],
    ) -> Result<String, Box<dyn std::error::Error>> {
        let key = format!("builds/{}/sdk.tar.gz", build_id);

        // Blocking upload
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .content_type("application/gzip")
            .body(sdk_bytes.to_vec().into())
            .send()
            .wait()?;

        Ok(format!("s3://{}/{}", self.bucket, key))
    }

    pub fn get_sdk_url(&self, build_id: &str) -> String {
        format!(
            "https://{}.s3.amazonaws.com/builds/{}/sdk.tar.gz",
            self.bucket, build_id
        )
    }
}
```

---

## 6. Caching on Lambda

### 6.1 In-Memory Cache (Per-Container)

```rust
// build_service/src/cache.rs
use std::collections::HashMap;
use once_cell::sync::Lazy;
use parking_lot::RwLock;

/// Global cache shared across invocations on same container
static RUSTDOC_CACHE: Lazy<RwLock<HashMap<String, Vec<u8>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

pub fn get_cached_rustdoc(crate_name: &str, version: &str) -> Option<Vec<u8>> {
    let key = format!("{}@{}", crate_name, version);
    RUSTDOC_CACHE.read().get(&key).cloned()
}

pub fn cache_rustdoc(crate_name: &str, version: &str, data: Vec<u8>) {
    let key = format!("{}@{}", crate_name, version);
    RUSTDOC_CACHE.write().insert(key, data);
}
```

### 6.2 DynamoDB for Cross-Container Cache

```rust
// build_service/src/dynamodb_cache.rs
use aws_sdk_dynamodb::Client as DynamoClient;

pub struct DynamoDbCache {
    client: DynamoClient,
    table: String,
}

impl DynamoDbCache {
    pub fn new(table: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let config = aws_config::from_env().load_sync();
        let client = DynamoClient::new(&config);

        Ok(Self {
            client,
            table: table.to_string(),
        })
    }

    pub fn get_sync(&self, key: &str) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
        let result = self
            .client
            .get_item()
            .table_name(&self.table)
            .primary_key("cache_key", key)
            .send()
            .wait()?;

        if let Some(item) = result.item {
            if let Some(attr) = item.get("data") {
                if let aws_sdk_dynamodb::primitives::Blob::Binary(data) = attr {
                    return Ok(Some(data.clone()));
                }
            }
        }

        Ok(None)
    }

    pub fn put_sync(
        &self,
        key: &str,
        data: &[u8],
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.client
            .put_item()
            .table_name(&self.table)
            .item("cache_key", key)
            .item("data", aws_sdk_dynamodb::primitives::Blob::new(data))
            .send()
            .wait()?;

        Ok(())
    }
}
```

---

## 7. Deployment Configuration

### 7.1 Cargo.toml for Lambda

```toml
[package]
name = "pavex-build-lambda"
version = "0.1.0"
edition = "2021"

[dependencies]
# Lambda runtime
lambda_runtime = "0.10"
aws_lambda_events = "0.15"

# AWS SDK (blocking)
aws-config = { version = "1.5", features = ["behavior-version-latest"] }
aws-sdk-s3 = "1.42"
aws-sdk-dynamodb = "1.40"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Valtron (no async!)
valtron = { path = "../../../ewe_platform/backends/foundation_core/src/valtron" }

# HTTP client (blocking)
reqwest = { version = "0.12", features = ["blocking", "json"] }

# Utilities
once_cell = "1.19"
parking_lot = "0.12"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["fmt"] }

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
```

### 7.2 Build Script for Lambda

```bash
#!/bin/bash
# scripts/build-lambda.sh

set -e

# Build for Lambda's x86_64 runtime
cargo build --release --target x86_64-unknown-linux-gnu

# Create bootstrap binary
cp target/x86_64-unknown-linux-gnu/release/pavex-build-lambda bootstrap

# Package for Lambda
zip -j lambda.zip bootstrap

echo "Build complete: lambda.zip"
```

### 7.3 Terraform Deployment

```hcl
# terraform/main.tf
resource "aws_lambda_function" "build_service" {
  filename         = "lambda.zip"
  function_name    = "pavex-build-service"
  role            = aws_iam_role.lambda_role.arn
  handler         = "bootstrap"
  runtime         = "provided.al2"
  timeout         = 30
  memory_size     = 1024

  environment {
    variables = {
      S3_BUCKET     = aws_s3_bucket.sdk_storage.id
      DYNAMODB_TABLE = aws_dynamodb_table.cache.name
    }
  }
}

resource "aws_api_gateway_rest_api" "api" {
  name = "pavex-build-api"
}

resource "aws_api_gateway_resource" "build" {
  rest_api_id = aws_api_gateway_rest_api.api.id
  parent_id   = aws_api_gateway_rest_api.api.root_resource_id
  path_part   = "build"
}

resource "aws_api_gateway_method" "post" {
  rest_api_id   = aws_api_gateway_rest_api.api.id
  resource_id   = aws_api_gateway_resource.build.id
  http_method   = "POST"
  authorization = "NONE"
}

resource "aws_api_gateway_integration" "lambda" {
  rest_api_id             = aws_api_gateway_rest_api.api.id
  resource_id             = aws_api_gateway_resource.build.id
  http_method             = aws_api_gateway_method.post.http_method
  integration_http_method = "POST"
  type                    = "AWS_PROXY"
  uri                     = aws_lambda_function.build_service.invoke_arn
}
```

---

## 8. Testing Strategies

### 8.1 Unit Tests (No Lambda)

```rust
// tests/build_task.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_task_completes() {
        let request = BuildRequest {
            crate_name: "test".to_string(),
            version: "0.1.0".to_string(),
            features: vec![],
        };

        let mut task = BuildTask::new(request);

        // Drive task to completion
        let mut status_count = 0;
        while let Some(_status) = task.next_status() {
            status_count += 1;
            assert!(status_count < 100, "Task stuck in loop");
        }

        assert!(task.result.is_some());
    }
}
```

### 8.2 Integration Tests (Local Lambda)

```bash
# Using AWS SAM Local
sam local invoke BuildFunction \
  --event test-event.json \
  --env-vars env.json
```

```json
// test-event.json
{
  "body": "{\"crate_name\":\"my_crate\",\"version\":\"0.1.0\",\"features\":[]}"
}
```

---

## Key Takeaways

1. **Valtron replaces tokio** - Iterator-based execution, no async runtime
2. **TaskIterator for build steps** - Explicit state machine for each build phase
3. **Lambda handler is sync** - NO async/await in your code
4. **Blocking HTTP client** - Use `reqwest::blocking` not `reqwest::async`
5. **S3/DynamoDB with .wait()** - Blocking AWS SDK calls
6. **Container-state caching** - In-memory cache persists across invocations
7. **DynamoDB for shared cache** - Cross-container rustdoc JSON caching
8. **Terraform for deployment** - Infrastructure as code

---

## Related Files

- **Valtron core**: `/home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron/`
- **TaskIterator spec**: `/home/darkvoid/Boxxed/@dev/ewe_platform/specifications/08-valtron-async-iterators/`
- **Fragment valtron guide**: `/home/darkvoid/Boxxed/@dev/repo-expolorations/alchemy/fragment/07-valtron-executor-guide.md`

---

*This completes the Pavex exploration. All documents are in `/home/darkvoid/Boxxed/@dev/repo-expolorations/pavex/`*
