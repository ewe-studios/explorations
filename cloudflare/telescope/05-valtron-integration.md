---
title: "Valtron Integration for Telescope"
subtitle: "Lambda deployment patterns for serverless filesystem backend without async/tokio"
---

# Valtron Integration for Telescope

## Introduction

This document covers integrating telescope with valtron executors for serverless Lambda deployment. The key constraint is **NO async/await, NO tokio** - using valtron's TaskIterator pattern instead.

## Table of Contents

1. [Valtron Executor Basics](#valtron-executor-basics)
2. [Lambda Runtime without Async](#lambda-runtime-without-async)
3. [HTTP API Compatibility](#http-api-compatibility)
4. [FS Backend Serverless](#fs-backend-serverless)
5. [Request/Response Types](#requestresponse-types)
6. [Production Deployment](#production-deployment)
7. [Complete Example](#complete-example)

---

## Valtron Executor Basics

### Understanding TaskIterator

Valtron replaces async/await with an iterator-based execution model:

```rust
use foundation_core::valtron::{
    TaskIterator,
    TaskStatus,
    FnReady,
    NoSpawner,
};

/// Basic TaskIterator example
struct SimpleTask {
    count: usize,
    max: usize,
}

impl TaskIterator for SimpleTask {
    /// Pending type: what we yield while working
    type Pending = ();

    /// Ready type: what we yield when complete
    type Ready = usize;

    /// Spawner type: for spawning sub-tasks
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        if self.count >= self.max {
            // Task complete - return Ready value
            Some(TaskStatus::Ready(self.count))
        } else {
            // Still working - return Pending
            self.count += 1;
            Some(TaskStatus::Pending(()))
        }
    }
}
```

### Single-Threaded Executor

```rust
use foundation_core::valtron::single::{
    initialize,
    spawn,
    run_until_complete,
    task_iter,
    block_iter,
};

/// Run tasks on single-threaded executor
pub fn run_single_threaded() -> Vec<usize> {
    // Initialize with seed (for reproducibility)
    initialize(42);

    let results = Rc::new(RefCell::new(Vec::new()));
    let results_clone = results.clone();

    // Spawn task
    spawn()
        .with_task(SimpleTask { count: 0, max: 5 })
        .with_resolver(Box::new(FnReady::new(move |item, _executor| {
            results_clone.borrow_mut().push(item);
        })))
        .schedule()
        .expect("should schedule");

    // Run to completion
    run_until_complete();

    Rc::try_unwrap(results).unwrap().into_inner()
}

/// Iterate over task status (non-blocking)
pub fn iterate_task_status() -> Vec<TaskStatus<usize, (), NoSpawner>> {
    initialize(42);

    let task = SimpleTask { count: 0, max: 5 };
    let mut statuses = Vec::new();

    for status in task_iter(spawn().with_task(task)) {
        statuses.push(status);
    }

    statuses
}

/// Block until task produces Ready value
pub fn blocking_iter_task() -> Vec<usize> {
    initialize(42);

    let task = SimpleTask { count: 0, max: 5 };
    let mut results = Vec::new();

    for value in block_iter(spawn().with_task(task)) {
        results.push(value);
    }

    results
}
```

### Multi-Threaded Executor

```rust
use foundation_core::valtron::multi::{
    block_on,
    get_pool,
};

/// Run tasks on multi-threaded executor
pub fn run_multi_threaded() {
    let seed = 42;

    block_on(seed, None, |pool| {
        pool.spawn()
            .with_task(SimpleTask { count: 0, max: 5 })
            .with_resolver(Box::new(FnReady::new(|item, _executor| {
                println!("Received: {}", item);
            })))
            .schedule()
            .expect("should schedule");
    });
}
```

---

## Lambda Runtime without Async

### AWS Lambda Runtime with Valtron

```rust
use aws_lambda_events::{
    alb::{AlbTargetGroupRequest, AlbTargetGroupResponse},
    encodings::Body,
};
use foundation_core::valtron::single::{initialize, spawn, run_until_complete};
use std::cell::RefCell;
use std::rc::Rc;

/// Lambda handler without async
pub fn handle_request(request: AlbTargetGroupRequest) -> Result<AlbTargetGroupResponse, LambdaError> {
    // Initialize valtron executor
    initialize(get_seed());

    // Create response cell for capturing result
    let response_cell = Rc::new(RefCell::new(None));
    let response_clone = response_cell.clone();

    // Spawn HTTP handler task
    spawn()
        .with_task(HttpHandlerTask::new(request))
        .with_resolver(Box::new(FnReady::new(move |response, _executor| {
            *response_clone.borrow_mut() = Some(response);
        })))
        .schedule()
        .expect("should schedule handler");

    // Run to completion (blocking)
    run_until_complete();

    // Extract response
    response_cell
        .borrow_mut()
        .take()
        .ok_or(LambdaError::NoResponse)
}

/// HTTP handler as TaskIterator
pub struct HttpHandlerTask {
    request: Option<AlbTargetGroupRequest>,
    state: HandlerState,
    response: Option<AlbTargetGroupResponse>,
}

enum HandlerState {
    Parsing,
    Routing,
    Processing,
    Responding,
    Complete,
}

impl HttpHandlerTask {
    pub fn new(request: AlbTargetGroupRequest) -> Self {
        Self {
            request: Some(request),
            state: HandlerState::Parsing,
            response: None,
        }
    }
}

impl TaskIterator for HttpHandlerTask {
    type Pending = ();
    type Ready = AlbTargetGroupResponse;
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        match self.state {
            HandlerState::Parsing => {
                // Parse request
                let _request = self.request.take().unwrap();
                self.state = HandlerState::Routing;
                Some(TaskStatus::Pending(()))
            }
            HandlerState::Routing => {
                // Route to handler
                self.state = HandlerState::Processing;
                Some(TaskStatus::Pending(()))
            }
            HandlerState::Processing => {
                // Process request (would spawn sub-tasks)
                self.state = HandlerState::Responding;
                Some(TaskStatus::Pending(()))
            }
            HandlerState::Responding => {
                // Build response
                self.response = Some(AlbTargetGroupResponse {
                    status_code: 200,
                    headers: Default::default(),
                    body: Some(Body::Text("OK".to_string())),
                    is_base64_encoded: false,
                });
                self.state = HandlerState::Complete;
                Some(TaskStatus::Pending(()))
            }
            HandlerState::Complete => {
                let response = self.response.take().unwrap();
                Some(TaskStatus::Ready(response))
            }
        }
    }
}

/// Generate seed from environment
fn get_seed() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
```

### Lambda Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum LambdaError {
    #[error("No response generated")]
    NoResponse,

    #[error("Task execution failed: {0}")]
    TaskFailed(String),

    #[error("Request parsing error: {0}")]
    RequestError(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<LambdaError> for AlbTargetGroupResponse {
    fn from(err: LambdaError) -> Self {
        AlbTargetGroupResponse {
            status_code: match err {
                LambdaError::RequestError(_) => 400,
                LambdaError::NoResponse | LambdaError::TaskFailed(_) => 500,
                LambdaError::Internal(_) => 500,
            },
            headers: Default::default(),
            body: Some(Body::Text(format!(r#"{{"error": "{}"}}"#, err))),
            is_base64_encoded: false,
        }
    }
}
```

---

## HTTP API Compatibility

### API Gateway Event Types

```rust
use aws_lambda_events::{
    apigw::{ApiGatewayV2HttpRequest, ApiGatewayV2HttpResponse},
    alb::{AlbTargetGroupRequest, AlbTargetGroupResponse},
};

/// Unified HTTP request type
pub enum HttpRequest {
    ApiGatewayV2(ApiGatewayV2HttpRequest),
    Alb(AlbTargetGroupRequest),
}

/// Unified HTTP response type
pub enum HttpResponse {
    ApiGatewayV2(ApiGatewayV2HttpResponse),
    Alb(AlbTargetGroupResponse),
}

impl HttpRequest {
    pub fn method(&self) -> &str {
        match self {
            HttpRequest::ApiGatewayV2(req) => &req.request_context.http.method,
            HttpRequest::Alb(req) => &req.http_method,
        }
    }

    pub fn path(&self) -> &str {
        match self {
            HttpRequest::ApiGatewayV2(req) => &req.raw_path,
            HttpRequest::Alb(req) => &req.path,
        }
    }

    pub fn body(&self) -> Option<&str> {
        match self {
            HttpRequest::ApiGatewayV2(req) => req.body.as_deref(),
            HttpRequest::Alb(req) => req.body.as_deref(),
        }
    }

    pub fn query_params(&self) -> std::collections::HashMap<String, String> {
        match self {
            HttpRequest::ApiGatewayV2(req) => req.query_string_parameters.clone(),
            HttpRequest::Alb(req) => req.query_string_parameters.clone(),
        }
    }
}

/// Router task for HTTP requests
pub struct HttpRouterTask {
    request: Option<HttpRequest>,
    state: RouterState,
}

enum RouterState {
    Parsing,
    Routing,
    NotFound,
    Complete,
}

impl TaskIterator for HttpRouterTask {
    type Pending = ();
    type Ready = HttpResponse;
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        match self.state {
            RouterState::Parsing => {
                let _request = self.request.take().unwrap();
                self.state = RouterState::Routing;
                Some(TaskStatus::Pending(()))
            }
            RouterState::Routing => {
                // Would match routes here
                self.state = RouterState::NotFound;
                Some(TaskStatus::Pending(()))
            }
            RouterState::NotFound => {
                self.state = RouterState::Complete;
                Some(TaskStatus::Ready(HttpResponse::not_found()))
            }
            RouterState::Complete => None,
        }
    }
}

impl HttpResponse {
    pub fn not_found() -> Self {
        HttpResponse::ApiGatewayV2(ApiGatewayV2HttpResponse {
            status_code: 404,
            headers: Default::default(),
            body: Some(Body::Text(r#"{"error": "Not Found"}"#.to_string())),
            is_base64_encoded: false,
        })
    }

    pub fn ok(body: &str) -> Self {
        HttpResponse::ApiGatewayV2(ApiGatewayV2HttpResponse {
            status_code: 200,
            headers: Default::default(),
            body: Some(Body::Text(body.to_string())),
            is_base64_encoded: false,
        })
    }

    pub fn error(status: u16, message: &str) -> Self {
        HttpResponse::ApiGatewayV2(ApiGatewayV2HttpResponse {
            status_code,
            headers: Default::default(),
            body: Some(Body::Text(format!(r#"{{"error": "{}"}}"#, message))),
            is_base64_encoded: false,
        })
    }
}
```

### JSON Request/Response

```rust
use serde::{Deserialize, Serialize};

/// API request types
#[derive(Debug, Deserialize)]
#[serde(tag = "action")]
pub enum ApiRequest {
    #[serde(rename = "create_test")]
    CreateTest(CreateTestRequest),
    #[serde(rename = "get_test")]
    GetTest(GetTestRequest),
    #[serde(rename = "list_tests")]
    ListTests(ListTestsRequest),
    #[serde(rename = "search_tests")]
    SearchTests(SearchTestsRequest),
    #[serde(rename = "delete_test")]
    DeleteTest(DeleteTestRequest),
}

#[derive(Debug, Deserialize)]
pub struct CreateTestRequest {
    url: String,
    browser: Option<String>,
    connection_type: Option<String>,
    timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct GetTestRequest {
    test_id: String,
}

#[derive(Debug, Deserialize)]
pub struct ListTestsRequest {
    limit: Option<usize>,
    offset: Option<usize>,
    browser: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SearchTestsRequest {
    query: String,
    filters: Option<SearchFilters>,
}

#[derive(Debug, Deserialize)]
pub struct SearchFilters {
    browser: Option<String>,
    date_from: Option<i64>,
    date_to: Option<i64>,
    slow_lcp: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteTestRequest {
    test_id: String,
}

/// API response types
#[derive(Debug, Serialize)]
#[serde(tag = "status")]
pub enum ApiResponse {
    #[serde(rename = "success")]
    Success { data: serde_json::Value },
    #[serde(rename = "error")]
    Error { message: String, code: Option<String> },
    #[serde(rename = "pending")]
    Pending { test_id: String },
}

impl ApiResponse {
    pub fn success(data: serde_json::Value) -> Self {
        ApiResponse::Success { data }
    }

    pub fn error(message: impl Into<String>) -> Self {
        ApiResponse::Error {
            message: message.into(),
            code: None,
        }
    }

    pub fn error_with_code(message: impl Into<String>, code: impl Into<String>) -> Self {
        ApiResponse::Error {
            message: message.into(),
            code: Some(code.into()),
        }
    }

    pub fn pending(test_id: impl Into<String>) -> Self {
        ApiResponse::Pending {
            test_id: test_id.into(),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|e| {
            format!(r#"{{"error": "Serialization failed: {}"}}"#, e)
        })
    }
}
```

---

## FS Backend Serverless

### S3-Compatible Storage Backend

```rust
use aws_sdk_s3::{
    Client as S3Client,
    config::Config,
    primitives::ByteStream,
};

/// Serverless storage backend using S3
pub struct ServerlessStorage {
    client: S3Client,
    bucket: String,
    prefix: String,
}

impl ServerlessStorage {
    pub fn new(client: S3Client, bucket: String, prefix: String) -> Self {
        Self { client, bucket, prefix }
    }

    /// Write operation as TaskIterator
    pub fn write_task(&self, key: String, data: Vec<u8>) -> S3WriteTask {
        S3WriteTask {
            client: Some(self.client.clone()),
            bucket: self.bucket.clone(),
            key: format!("{}/{}", self.prefix, key),
            data: Some(data),
            state: WriteState::Initializing,
            result: None,
        }
    }
}

/// S3 write task
pub struct S3WriteTask {
    client: Option<S3Client>,
    bucket: String,
    key: String,
    data: Option<Vec<u8>>,
    state: WriteState,
    result: Option<Result<(), S3Error>>,
}

enum WriteState {
    Initializing,
    Uploading,
    Complete,
}

impl TaskIterator for S3WriteTask {
    type Pending = ();
    type Ready = Result<(), S3Error>;
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        match self.state {
            WriteState::Initializing => {
                self.state = WriteState::Uploading;
                Some(TaskStatus::Pending(()))
            }
            WriteState::Uploading => {
                // In real implementation, this would use non-blocking S3 client
                // For valtron, we'd need to implement proper async integration
                let client = self.client.take().unwrap();
                let data = self.data.take().unwrap();

                // Synchronous S3 upload (Lambda has time limits)
                let rt = tokio::runtime::Runtime::new().unwrap();
                let result = rt.block_on(async {
                    client
                        .put_object()
                        .bucket(&self.bucket)
                        .key(&self.key)
                        .body(ByteStream::from(data))
                        .send()
                        .await
                        .map(|_| ())
                        .map_err(|e| S3Error::UploadFailed(e.to_string()))
                });

                self.result = Some(result);
                self.state = WriteState::Complete;
                Some(TaskStatus::Pending(()))
            }
            WriteState::Complete => {
                let result = self.result.take().unwrap();
                Some(TaskStatus::Ready(result))
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum S3Error {
    #[error("Upload failed: {0}")]
    UploadFailed(String),
    #[error("Download failed: {0}")]
    DownloadFailed(String),
    #[error("Not found: {0}")]
    NotFound(String),
}
```

### In-Memory Index for Lambda

```rust
use std::collections::{HashMap, BTreeMap};

/// Serverless index (stored in Lambda memory)
pub struct ServerlessIndex {
    /// Path -> Entry
    by_path: HashMap<String, IndexEntry>,
    /// Timestamp -> Path (for time-based queries)
    by_timestamp: BTreeMap<i64, String>,
    /// URL -> Paths (for URL queries)
    by_url: HashMap<String, Vec<String>>,
    /// Dirty flag
    dirty: bool,
}

#[derive(Debug, Clone)]
pub struct IndexEntry {
    pub test_id: String,
    pub path: String,
    pub url: String,
    pub browser: String,
    pub timestamp: i64,
    pub status: String,
    pub metrics: TestMetricsSnapshot,
}

#[derive(Debug, Clone)]
pub struct TestMetricsSnapshot {
    pub lcp_ms: Option<f64>,
    pub cls: Option<f64>,
    pub ttfb_ms: Option<f64>,
}

impl ServerlessIndex {
    pub fn new() -> Self {
        Self {
            by_path: HashMap::new(),
            by_timestamp: BTreeMap::new(),
            by_url: HashMap::new(),
            dirty: false,
        }
    }

    /// Add entry to index
    pub fn add(&mut self, entry: IndexEntry) {
        self.by_path.insert(entry.path.clone(), entry.clone());
        self.by_timestamp.insert(entry.timestamp, entry.path.clone());
        self.by_url
            .entry(entry.url.clone())
            .or_insert_with(Vec::new)
            .push(entry.path.clone());
        self.dirty = true;
    }

    /// Get entry by path
    pub fn get(&self, path: &str) -> Option<&IndexEntry> {
        self.by_path.get(path)
    }

    /// Search by URL prefix
    pub fn search_by_url(&self, url_prefix: &str) -> Vec<&IndexEntry> {
        self.by_path
            .values()
            .filter(|e| e.url.contains(url_prefix))
            .collect()
    }

    /// Search by time range
    pub fn search_by_time_range(&self, start: i64, end: i64) -> Vec<&IndexEntry> {
        self.by_timestamp
            .range(start..=end)
            .filter_map(|(_, path)| self.by_path.get(path))
            .collect()
    }

    /// Search by browser
    pub fn search_by_browser(&self, browser: &str) -> Vec<&IndexEntry> {
        self.by_path
            .values()
            .filter(|e| &e.browser == browser)
            .collect()
    }

    /// Mark as clean (persisted)
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    /// Check if dirty
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
}

impl Default for ServerlessIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// Index persistence task
pub struct IndexPersistTask {
    index: Rc<RefCell<ServerlessIndex>>,
    storage: Rc<ServerlessStorage>,
    state: PersistState,
    result: Option<Result<(), PersistError>>,
}

enum PersistState {
    Serializing,
    Uploading,
    Complete,
}

impl TaskIterator for IndexPersistTask {
    type Pending = ();
    type Ready = Result<(), PersistError>;
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        match self.state {
            PersistState::Serializing => {
                // Serialize index
                let index = self.index.borrow();
                let data = serde_json::to_vec(&*index).unwrap();

                // Trigger upload
                self.state = PersistState::Uploading;
                Some(TaskStatus::Pending(()))
            }
            PersistState::Uploading => {
                // Upload to S3
                // In real impl, would use S3WriteTask
                self.state = PersistState::Complete;
                Some(TaskStatus::Pending(()))
            }
            PersistState::Complete => {
                let result = self.result.take().unwrap();
                Some(TaskStatus::Ready(result))
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PersistError {
    #[error("Serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Upload failed: {0}")]
    Upload(#[from] S3Error),
}
```

---

## Request/Response Types

### Lambda Event Structures

```rust
use aws_lambda_events::encodings::Body;
use serde::{Deserialize, Serialize};

/// Telescope Lambda request
#[derive(Debug, Deserialize)]
pub struct TelescopeRequest {
    #[serde(flatten)]
    pub event: LambdaEvent,
    #[serde(default)]
    pub context: RequestContext,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum LambdaEvent {
    ApiGateway(ApiGatewayV2HttpRequest),
    Alb(AlbTargetGroupRequest),
    Scheduled(ScheduledEvent),
}

#[derive(Debug, Deserialize, Default)]
pub struct ScheduledEvent {
    pub source: Option<String>,
    pub time: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct RequestContext {
    pub test_id: Option<String>,
    pub async_execution: Option<bool>,
}

/// Telescope Lambda response
#[derive(Debug, Serialize)]
pub struct TelescopeResponse {
    pub status_code: u16,
    pub headers: std::collections::HashMap<String, String>,
    pub body: String,
    pub is_base64_encoded: bool,
}

impl TelescopeResponse {
    pub fn json(status_code: u16, body: serde_json::Value) -> Self {
        let mut headers = std::collections::HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        Self {
            status_code,
            headers,
            body: body.to_string(),
            is_base64_encoded: false,
        }
    }

    pub fn success<T: Serialize>(data: T) -> Self {
        Self::json(200, serde_json::json!({
            "status": "success",
            "data": data,
        }))
    }

    pub fn error(status_code: u16, message: impl Into<String>) -> Self {
        Self::json(status_code, serde_json::json!({
            "status": "error",
            "message": message.into(),
        }))
    }

    pub fn accepted(test_id: &str) -> Self {
        Self::json(202, serde_json::json!({
            "status": "pending",
            "test_id": test_id,
        }))
    }
}
```

### Test Configuration for Lambda

```rust
/// Lambda-specific test configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LambdaTestConfig {
    pub url: String,
    pub browser: BrowserName,
    pub viewport: ViewportConfig,
    pub network: Option<NetworkConfig>,
    pub timeout_ms: u64,
    pub storage: StorageConfig,
    pub async_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewportConfig {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub connection_type: ConnectionType,
    pub cpu_throttle: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub backend: StorageBackend,
    pub bucket: String,
    pub prefix: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageBackend {
    #[serde(rename = "s3")]
    S3,
    #[serde(rename = "memory")]
    Memory,
}

impl Default for LambdaTestConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            browser: BrowserName::Chrome,
            viewport: ViewportConfig {
                width: 1366,
                height: 768,
            },
            network: None,
            timeout_ms: 30000,
            storage: StorageConfig {
                backend: StorageBackend::S3,
                bucket: String::new(),
                prefix: String::new(),
            },
            async_mode: false,
        }
    }
}
```

---

## Production Deployment

### CloudFormation Template

```yaml
AWSTemplateFormatVersion: '2010-09-09'
Description: Telescope Serverless FS Backend

Resources:
  # S3 Bucket for test results
  ResultsBucket:
    Type: AWS::S3::Bucket
    Properties:
      BucketName: !Sub 'telescope-results-${AWS::AccountId}'
      LifecycleConfiguration:
        Rules:
          - Id: ExpireOldResults
            Status: Enabled
            ExpirationInDays: 30

  # DynamoDB table for index
  IndexTable:
    Type: AWS::DynamoDB::Table
    Properties:
      TableName: telescope-index
      AttributeDefinitions:
        - AttributeName: test_id
          AttributeType: S
        - AttributeName: url
          AttributeType: S
        - AttributeName: timestamp
          AttributeType: N
      KeySchema:
        - AttributeName: test_id
          KeyType: HASH
      GlobalSecondaryIndexes:
        - IndexName: url-index
          KeySchema:
            - AttributeName: url
              KeyType: HASH
            - AttributeName: timestamp
              KeyType: RANGE
          Projection:
            ProjectionType: ALL

  # Lambda function
  TelescopeFunction:
    Type: AWS::Lambda::Function
    Properties:
      FunctionName: telescope-handler
      Runtime: provided.al2
      Handler: bootstrap
      MemorySize: 1024
      Timeout: 30
      Environment:
        Variables:
          RESULTS_BUCKET: !Ref ResultsBucket
          INDEX_TABLE: !Ref IndexTable
      Role: !GetAtt LambdaExecutionRole.Arn
      Code:
        S3Bucket: !Ref DeploymentBucket
        S3Key: telescope-lambda.zip

  # API Gateway
  ApiGateway:
    Type: AWS::ApiGatewayV2::Api
    Properties:
      Name: telescope-api
      ProtocolType: HTTP
      Description: Telescope API

  # API Gateway integration
  ApiIntegration:
    Type: AWS::ApiGatewayV2::Integration
    Properties:
      ApiId: !Ref ApiGateway
      IntegrationType: AWS_PROXY
      IntegrationUri: !GetAtt TelescopeFunction.Arn
      IntegrationMethod: POST
      PayloadFormatVersion: '2.0'

  # Routes
  CreateTestRoute:
    Type: AWS::ApiGatewayV2::Route
    Properties:
      ApiId: !Ref ApiGateway
      RouteKey: POST /tests
      Target: !Sub 'integrations/${ApiIntegration}'

  GetTestRoute:
    Type: AWS::ApiGatewayV2::Route
    Properties:
      ApiId: !Ref ApiGateway
      RouteKey: GET /tests/{test_id}
      Target: !Sub 'integrations/${ApiIntegration}'

  ListTestsRoute:
    Type: AWS::ApiGatewayV2::Route
    Properties:
      ApiId: !Ref ApiGateway
      RouteKey: GET /tests
      Target: !Sub 'integrations/${ApiIntegration}'

  # Permissions
  LambdaExecutionRole:
    Type: AWS::IAM::Role
    Properties:
      AssumeRolePolicyDocument:
        Version: '2012-10-17'
        Statement:
          - Effect: Allow
            Principal:
              Service: lambda.amazonaws.com
            Action: sts:AssumeRole
      ManagedPolicyArns:
        - arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole
      Policies:
        - PolicyName: TelescopeAccess
          PolicyDocument:
            Version: '2012-10-17'
            Statement:
              - Effect: Allow
                Action:
                  - s3:GetObject
                  - s3:PutObject
                  - s3:DeleteObject
                  - s3:ListBucket
                Resource:
                  - !Sub '${ResultsBucket}/*'
                  - !Sub '${ResultsBucket}'
              - Effect: Allow
                Action:
                  - dynamodb:GetItem
                  - dynamodb:PutItem
                  - dynamodb:Query
                  - dynamodb:Scan
                Resource:
                  - !Sub '${IndexTable}'
                  - !Sub '${IndexTable}/*'

Outputs:
  ApiEndpoint:
    Description: API Gateway endpoint URL
    Value: !Sub 'https://${ApiGateway}.execute-api.${AWS::Region}.amazonaws.com'
```

### Deployment Script

```bash
#!/bin/bash
set -e

# Build Rust binary for Lambda
cargo build --release --target x86_64-unknown-linux-musl

# Create deployment package
mkdir -p deployment
cp target/x86_64-unknown-linux-musl/release/telescope-lambda deployment/bootstrap
cd deployment
zip -r ../telescope-lambda.zip .
cd ..

# Deploy to S3
aws s3 cp telescope-lambda.zip s3://telescope-deployments/telescope-lambda.zip

# Update Lambda function
aws lambda update-function-code \
    --function-name telescope-handler \
    --s3-bucket telescope-deployments \
    --s3-key telescope-lambda.zip

# Deploy CloudFormation
aws cloudformation deploy \
    --template-file cloudformation.yml \
    --stack-name telescope-serverless \
    --capabilities CAPABILITY_IAM
```

---

## Complete Example

### Main Lambda Entry Point

```rust
//! Telescope Lambda Handler
//!
//! Serverless filesystem backend using valtron executors
//! NO async/await, NO tokio in the main execution path

use aws_lambda_events::alb::{AlbTargetGroupRequest, AlbTargetGroupResponse};
use foundation_core::valtron::single::{initialize, spawn, run_until_complete};
use std::cell::RefCell;
use std::rc::Rc;

mod handler;
mod storage;
mod index;
mod types;

use handler::HttpHandlerTask;
use storage::ServerlessStorage;
use index::ServerlessIndex;
use types::{LambdaTestConfig, TelescopeResponse};

/// Global state (Lambda reuses instances)
static mut STORAGE: Option<Rc<ServerlessStorage>> = None;
static mut INDEX: Option<Rc<RefCell<ServerlessIndex>>> = None;

/// Main Lambda handler
#[no_mangle]
pub extern "C" fn handler(event: AlbTargetGroupRequest) -> AlbTargetGroupResponse {
    // Initialize valtron executor
    initialize(get_seed());

    // Get or create global state
    let storage = unsafe {
        STORAGE.get_or_insert_with(|| {
            let client = create_s3_client();
            let bucket = std::env::var("RESULTS_BUCKET").unwrap();
            Rc::new(ServerlessStorage::new(client, bucket, "results".to_string()))
        }).clone()
    };

    let index = unsafe {
        INDEX.get_or_insert_with(|| {
            Rc::new(RefCell::new(ServerlessIndex::new()))
        }).clone()
    };

    // Create response cell
    let response_cell = Rc::new(RefCell::new(None));
    let response_clone = response_cell.clone();

    // Spawn handler task
    spawn()
        .with_task(HttpHandlerTask::new(event, storage, index))
        .with_resolver(Box::new(FnReady::new(move |response, _executor| {
            *response_clone.borrow_mut() = Some(response);
        })))
        .schedule()
        .expect("should schedule handler");

    // Run to completion
    run_until_complete();

    // Extract and return response
    response_cell
        .borrow_mut()
        .take()
        .unwrap_or_else(|| error_response("No response generated"))
}

fn create_s3_client() -> aws_sdk_s3::Client {
    let config = aws_config::load_from_env();
    aws_sdk_s3::Client::new(&config)
}

fn get_seed() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn error_response(message: &str) -> AlbTargetGroupResponse {
    use aws_lambda_events::encodings::Body;
    AlbTargetGroupResponse {
        status_code: 500,
        headers: Default::default(),
        body: Some(Body::Text(format!(r#"{{"error": "{}"}}"#, message))),
        is_base64_encoded: false,
    }
}

// Required for Rust lambda runtime
fn main() {}
```

### Handler Implementation

```rust
use aws_lambda_events::alb::{AlbTargetGroupRequest, AlbTargetGroupResponse};
use aws_lambda_events::encodings::Body;
use foundation_core::valtron::{TaskIterator, TaskStatus, FnReady, NoSpawner};
use serde_json::json;
use std::cell::RefCell;
use std::rc::Rc;

use crate::storage::ServerlessStorage;
use crate::index::ServerlessIndex;
use crate::types::{ApiRequest, TelescopeResponse, LambdaTestConfig, CreateTestRequest};

/// HTTP Handler Task
pub struct HttpHandlerTask {
    request: Option<AlbTargetGroupRequest>,
    storage: Rc<ServerlessStorage>,
    index: Rc<RefCell<ServerlessIndex>>,
    state: HandlerState,
    response: Option<AlbTargetGroupResponse>,
}

enum HandlerState {
    Parsing,
    Routing,
    Processing,
    Responding,
    Complete,
}

impl HttpHandlerTask {
    pub fn new(
        request: AlbTargetGroupRequest,
        storage: Rc<ServerlessStorage>,
        index: Rc<RefCell<ServerlessIndex>>,
    ) -> Self {
        Self {
            request: Some(request),
            storage,
            index,
            state: HandlerState::Parsing,
            response: None,
        }
    }

    fn parse_request(&self) -> Result<ApiRequest, String> {
        let request = self.request.as_ref().unwrap();
        let body = request.body.as_ref().ok_or("Missing body")?;
        let api_request: ApiRequest = serde_json::from_str(body)
            .map_err(|e| format!("Invalid JSON: {}", e))?;
        Ok(api_request)
    }
}

impl TaskIterator for HttpHandlerTask {
    type Pending = ();
    type Ready = AlbTargetGroupResponse;
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        match self.state {
            HandlerState::Parsing => {
                // Parse incoming request
                let _api_request = self.parse_request();
                self.state = HandlerState::Routing;
                Some(TaskStatus::Pending(()))
            }
            HandlerState::Routing => {
                // Route to appropriate handler
                match self.parse_request() {
                    Ok(ApiRequest::CreateTest(req)) => {
                        // Queue test creation
                        self.state = HandlerState::Processing;
                    }
                    Ok(ApiRequest::GetTest(req)) => {
                        // Get test results
                        self.state = HandlerState::Processing;
                    }
                    Ok(ApiRequest::ListTests(req)) => {
                        // List tests
                        self.state = HandlerState::Processing;
                    }
                    Err(e) => {
                        self.response = Some(TelescopeResponse::error(400, e).into());
                        self.state = HandlerState::Responding;
                    }
                }
                Some(TaskStatus::Pending(()))
            }
            HandlerState::Processing => {
                // Process request based on route
                let api_request = self.parse_request().unwrap();

                match api_request {
                    ApiRequest::CreateTest(req) => {
                        let test_id = generate_test_id();

                        // Spawn test execution task
                        let test_config = LambdaTestConfig {
                            url: req.url,
                            browser: req.browser.and_then(|b| b.parse().ok()).unwrap_or_default(),
                            ..Default::default()
                        };

                        // In real impl, would spawn test and store state
                        self.response = Some(TelescopeResponse::accepted(&test_id).into());
                    }
                    ApiRequest::GetTest(req) => {
                        // Get test from index
                        let index = self.index.borrow();
                        if let Some(entry) = index.get(&req.test_id) {
                            self.response = Some(TelescopeResponse::success(json!(entry)).into());
                        } else {
                            self.response = Some(TelescopeResponse::error(404, "Test not found").into());
                        }
                    }
                    ApiRequest::ListTests(req) => {
                        let index = self.index.borrow();
                        let tests: Vec<_> = index.by_path.values().take(req.limit.unwrap_or(100)).collect();
                        self.response = Some(TelescopeResponse::success(json!({
                            "tests": tests,
                            "total": tests.len(),
                        })).into());
                    }
                    _ => {
                        self.response = Some(TelescopeResponse::error(400, "Unknown request").into());
                    }
                }

                self.state = HandlerState::Responding;
                Some(TaskStatus::Pending(()))
            }
            HandlerState::Responding => {
                self.state = HandlerState::Complete;
                Some(TaskStatus::Pending(()))
            }
            HandlerState::Complete => {
                let response = self.response.take()
                    .unwrap_or_else(|| error_response("No response"));
                Some(TaskStatus::Ready(response))
            }
        }
    }
}

fn generate_test_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    use uuid::Uuid;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();

    format!("{}_{}", now, Uuid::new_v4().to_string().replace("-", ""))
}

fn error_response(message: &str) -> AlbTargetGroupResponse {
    AlbTargetGroupResponse {
        status_code: 500,
        headers: Default::default(),
        body: Some(Body::Text(format!(r#"{{"error": "{}"}}"#, message))),
        is_base64_encoded: false,
    }
}

impl From<TelescopeResponse> for AlbTargetGroupResponse {
    fn from(response: TelescopeResponse) -> Self {
        AlbTargetGroupResponse {
            status_code: response.status_code,
            headers: response.headers,
            body: Some(Body::Text(response.body)),
            is_base64_encoded: response.is_base64_encoded,
        }
    }
}
```

---

## Summary

| Topic | Key Points |
|-------|------------|
| Valtron | TaskIterator replaces async/await |
| Lambda Runtime | Single-threaded executor with blocking run_until_complete |
| HTTP API | API Gateway and ALB event handling |
| Storage | S3-compatible serverless storage |
| Index | In-memory index with persistence |
| Deployment | CloudFormation, musl build target |

---

## Exploration Complete

This completes the comprehensive telescope exploration. Refer to [exploration.md](exploration.md) for the full table of contents and navigation.
