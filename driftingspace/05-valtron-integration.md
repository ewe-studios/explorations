---
title: "Valtron Integration: Lambda Deployment"
subtitle: "Deploying Aper to AWS Lambda without async/tokio"
prerequisites: [production-grade.md](production-grade.md)
valtron_docs: /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron/
---

# Valtron Integration: Lambda Deployment

This document explains how to deploy Aper-based applications to AWS Lambda using Valtron executors, **without async/await or tokio**.

## Table of Contents

1. [Why Valtron?](#1-why-valtron)
2. [Valtron Executor Basics](#2-valtron-executor-basics)
3. [TaskIterator Pattern](#3-taskiterator-pattern)
4. [HTTP API Handler](#4-http-api-handler)
5. [WebSocket to HTTP Adaptation](#5-websocket-to-http-adaptation)
6. [Stateless Lambda Pattern](#6-stateless-lambda-pattern)
7. [Complete Lambda Implementation](#7-complete-lambda-implementation)
8. [Deployment Configuration](#8-deployment-configuration)

---

## 1. Why Valtron?

### The Problem with async/await on Lambda

Standard Lambda runtimes use async runtimes that don't fit all use cases:

```rust
// Typical async Lambda (requires tokio)
use aws_lambda_events::event::ApiGatewayV2HttpRequestEvent;
use lambda_runtime::{service_fn, Error, LambdaEvent};

async fn handler(event: LambdaEvent<ApiGatewayV2HttpRequestEvent>) -> Result<(), Error> {
    // Uses tokio runtime
    // Not suitable for sync state machine processing
}
```

### Valtron Advantages

| Feature | Valtron | aws-lambda-rust-runtime |
|---------|---------|------------------------|
| Runtime | Single-threaded executor | Tokio async runtime |
| Memory | Lower overhead | Higher overhead |
| Cold Start | Faster | Slower |
| State Machines | Natural fit | Requires adaptation |
| Learning Curve | Rust idioms | Async patterns |

### Valtron Philosophy

Valtron uses **TaskIterator** pattern instead of async/await:

```rust
// Instead of async:
async fn fetch_data() -> Result<Data> { ... }

// Use TaskIterator:
struct FetchTask { url: String }
impl TaskIterator for FetchTask {
    type Ready = Data;
    type Pending = ();

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending>> {
        // Return Pending or Ready
    }
}
```

---

## 2. Valtron Executor Basics

### Executor Types

Valtron provides two executor types:

**Single-threaded Executor**:
```rust
use valtron::executor::Executor;

let mut executor = Executor::new();
executor.spawn(task);
executor.run();
```

**Multi-threaded Executor**:
```rust
use valtron::executor::MultiThreadedExecutor;

let executor = MultiThreadedExecutor::new(4); // 4 threads
executor.spawn(task);
executor.run();
```

### TaskIterator Trait

```rust
pub trait TaskIterator {
    type Ready;
    type Pending;
    type Spawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending>>;
}
```

### TaskStatus

```rust
pub enum TaskStatus<Ready, Pending> {
    /// Task is complete with result
    Ready(Ready),
    /// Task is still running
    Pending(Pending),
    /// Task spawned another task
    Spawned(Self::Spawner),
}
```

---

## 3. TaskIterator Pattern

### Basic Task Implementation

```rust
use valtron::task::{TaskIterator, TaskStatus};

pub struct ProcessTransitionTask<S: StateMachine> {
    state: S,
    transition: Option<S::Transition>,
    result: Option<Result<S, S::Conflict>>,
}

impl<S: StateMachine> ProcessTransitionTask<S> {
    pub fn new(state: S, transition: S::Transition) -> Self {
        Self {
            state,
            transition: Some(transition),
            result: None,
        }
    }
}

impl<S: StateMachine> TaskIterator for ProcessTransitionTask<S> {
    type Ready = Result<S, S::Conflict>;
    type Pending = ();
    type Spawner = ();

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending>> {
        if let Some(transition) = self.transition.take() {
            // Apply transition
            let result = self.state.apply(&transition);
            self.result = Some(result.clone());
            Some(TaskStatus::Ready(result))
        } else {
            None // Task complete
        }
    }
}
```

### DrivenRecvIterator for I/O

For I/O operations that need polling:

```rust
use valtron::task::DrivenRecvIterator;

pub struct HttpFetchTask {
    url: String,
    receiver: DrivenRecvIterator<Vec<u8>>,
}

impl HttpFetchTask {
    pub fn new(url: String) -> Self {
        let (sender, receiver) = DrivenRecvIterator::new();

        // Start HTTP request in background (using minimal HTTP client)
        std::thread::spawn(move || {
            let response = ureq::get(&url).call();
            if let Ok(response) = response {
                let bytes = response.into_bytes().to_vec();
                sender.send(bytes);
            }
        });

        Self { url, receiver }
    }
}

impl TaskIterator for HttpFetchTask {
    type Ready = Vec<u8>;
    type Pending = ();
    type Spawner = ();

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending>> {
        // Poll the receiver
        match self.receiver.next() {
            Some(bytes) => Some(TaskStatus::Ready(bytes)),
            None => Some(TaskStatus::Pending(())),
        }
    }
}
```

---

## 4. HTTP API Handler

### Lambda Event Types

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ApiGatewayV2HttpRequestEvent {
    pub version: String,
    pub route_key: String,
    pub raw_path: String,
    pub raw_query_string: String,
    pub headers: std::collections::HashMap<String, String>,
    pub body: Option<String>,
    pub is_base64_encoded: bool,
    pub request_context: ApiGatewayV2RequestContext,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ApiGatewayV2RequestContext {
    pub http: Http,
    pub connection_id: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Http {
    pub method: String,
    pub path: String,
}
```

### Response Types

```rust
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize)]
pub struct ApiGatewayV2HttpResponse {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub is_base64_encoded: bool,
}

impl ApiGatewayV2HttpResponse {
    pub fn ok(body: String) -> Self {
        Self {
            status_code: 200,
            headers: [("content-type".to_string(), "application/json".to_string())]
                .iter().cloned().collect(),
            body,
            is_base64_encoded: false,
        }
    }

    pub fn error(status: u16, message: String) -> Self {
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "application/json".to_string());
        headers.insert("access-control-allow-origin".to_string(), "*".to_string());

        Self {
            status_code: status,
            headers,
            body: serde_json::json!({ "error": message }).to_string(),
            is_base64_encoded: false,
        }
    }
}
```

### Valtron-Based Handler

```rust
use valtron::executor::Executor;
use valtron::task::TaskIterator;

pub struct LambdaHandler<S: StateMachine> {
    state_store: StatePersistence<S>,
    executor: Executor,
}

impl<S: StateMachine + 'static> LambdaHandler<S> {
    pub fn new(table_name: String) -> Self {
        Self {
            state_store: StatePersistence::new(&table_name),
            executor: Executor::new(),
        }
    }

    pub fn handle(&mut self, event: ApiGatewayV2HttpRequestEvent) -> ApiGatewayV2HttpResponse {
        // Parse request
        let (action, body) = match self.parse_request(&event) {
            Ok(parsed) => parsed,
            Err(e) => return ApiGatewayV2HttpResponse::error(400, e),
        };

        // Create task for processing
        let task = ProcessLambdaRequestTask::new(
            action,
            body,
            self.state_store.clone(),
        );

        // Spawn and run synchronously
        self.executor.spawn(task);

        // Run executor until all tasks complete
        // For Lambda, we expect single request/response
        let result = self.executor.run_once();

        // Convert result to HTTP response
        self.to_http_response(result)
    }

    fn parse_request(&self, event: &ApiGatewayV2HttpRequestEvent) -> Result<(String, String), String> {
        // Extract action from path or body
        let path = &event.raw_path;
        let body = event.body.clone().unwrap_or_default();

        if path.contains("/transition") {
            Ok(("apply_transition".to_string(), body))
        } else if path.contains("/state") {
            Ok(("get_state".to_string(), body))
        } else {
            Err(format!("Unknown path: {}", path))
        }
    }

    fn to_http_response(&self, result: serde_json::Value) -> ApiGatewayV2HttpResponse {
        ApiGatewayV2HttpResponse::ok(result.to_string())
    }
}
```

---

## 5. WebSocket to HTTP Adaptation

Lambda doesn't natively support persistent WebSocket connections for long-running sessions. We adapt using API Gateway WebSocket API:

### Connection Management

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WebSocketConnectEvent {
    pub connection_id: String,
    pub domain_name: String,
    pub event_type: String, // "CONNECT", "MESSAGE", "DISCONNECT"
    pub body: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WebSocketMessageEvent {
    pub connection_id: String,
    pub domain_name: String,
    pub event_type: String,
    pub body: Option<String>,
}
```

### State Store with DynamoDB

```rust
use rusoto_dynamodb::{DynamoDb, DynamoDbClient, GetItemInput, PutItemInput};
use rusoto_core::Region;

pub struct DynamoDbStateStore<S: StateMachine> {
    client: DynamoDbClient,
    table_name: String,
    _phantom: PhantomData<S>,
}

impl<S: StateMachine> DynamoDbStateStore<S> {
    pub fn new(table_name: String) -> Self {
        let client = DynamoDbClient::new(Region::UsEast1);
        Self {
            client,
            table_name,
            _phantom: PhantomData,
        }
    }

    pub async fn get_state(&self, connection_id: &str) -> Result<Option<(S, StateVersionNumber)>, String> {
        let input = GetItemInput {
            table_name: self.table_name.clone(),
            key: [("connection_id".to_string(), connection_id.to_string().into_attr())]
                .iter().cloned().collect(),
            ..Default::default()
        };

        // Note: In Valtron, use sync HTTP client instead of async
        // This is pseudocode - actual implementation uses ureq or similar
        todo!("Implement sync DynamoDB access")
    }

    pub fn save_state(&self, connection_id: &str, state: &S, version: StateVersionNumber) -> Result<(), String> {
        // Save state to DynamoDB
        todo!("Implement sync DynamoDB access")
    }
}
```

### Message Processing Task

```rust
pub struct ProcessWebSocketMessageTask<S: StateMachine> {
    connection_id: String,
    message: String,
    state_store: DynamoDbStateStore<S>,
    response: Option<ApiGatewayV2HttpResponse>,
}

impl<S: StateMachine + 'static> ProcessWebSocketMessageTask<S> {
    pub fn new(connection_id: String, message: String, state_store: DynamoDbStateStore<S>) -> Self {
        Self {
            connection_id,
            message,
            state_store,
            response: None,
        }
    }
}

impl<S: StateMachine + 'static> TaskIterator for ProcessWebSocketMessageTask<S> {
    type Ready = ApiGatewayV2HttpResponse;
    type Pending = ();
    type Spawner = ();

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending>> {
        // Load state
        let (state, version) = match self.state_store.get_state(&self.connection_id) {
            Ok(Some(s)) => s,
            Ok(None) => {
                // Initialize new state
                (S::new(), StateVersionNumber(0))
            }
            Err(e) => {
                return Some(TaskStatus::Ready(
                    ApiGatewayV2HttpResponse::error(500, e)
                ));
            }
        };

        // Parse and apply transition
        let transition: MessageToServer<S> = match serde_json::from_str(&self.message) {
            Ok(t) => t,
            Err(e) => {
                return Some(TaskStatus::Ready(
                    ApiGatewayV2HttpResponse::error(400, format!("Invalid message: {}", e))
                ));
            }
        };

        // Process transition
        let response = self.process_transition(state, version, transition);
        Some(TaskStatus::Ready(response))
    }
}
```

---

## 6. Stateless Lambda Pattern

For true serverless, use stateless pattern with external state store:

### Architecture

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   Lambda 1   │     │   Lambda 2   │     │   Lambda 3   │
│              │     │              │     │              │
│  Valtron     │     │  Valtron     │     │  Valtron     │
│  Executor    │     │  Executor    │     │  Executor    │
└──────┬───────┘     └──────┬───────┘     └──────┬───────┘
       │                    │                    │
       └────────────────────┼────────────────────┘
                            │
                     ┌──────▼───────┐
                     │  DynamoDB    │
                     │  (State)     │
                     └──────────────┘
```

### Request/Response Flow

```
1. Client sends WebSocket message to API Gateway
2. API Gateway triggers Lambda
3. Lambda loads state from DynamoDB
4. Valtron processes transition
5. Lambda saves state to DynamoDB
6. Lambda broadcasts to other connections via API Gateway Management API
7. Lambda returns response
```

### Connection State

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConnectionState<S: StateMachine> {
    pub state: S,
    pub version: StateVersionNumber,
    pub client_id: ClientId,
    pub pending_transitions: Vec<(ClientTransitionNumber, S::Transition)>,
    pub last_activity: u64, // Unix timestamp
}

impl<S: StateMachine> ConnectionState<S> {
    pub fn new(state: S) -> Self {
        Self {
            state,
            version: StateVersionNumber(0),
            client_id: generate_client_id(),
            pending_transitions: Vec::new(),
            last_activity: current_timestamp(),
        }
    }
}
```

---

## 7. Complete Lambda Implementation

### Main Handler

```rust
use valtron::executor::Executor;
use valtron::task::TaskIterator;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
pub struct LambdaEvent {
    #[serde(flatten)]
    pub event: LambdaEventType,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "requestContext", content = "event_type")]
pub enum LambdaEventType {
    #[serde(rename = "MESSAGE")]
    WebSocketMessage(WebSocketMessageEvent),
    #[serde(rename = "CONNECT")]
    WebSocketConnect(WebSocketConnectEvent),
    #[serde(rename = "http")]
    Http(ApiGatewayV2HttpRequestEvent),
}

pub struct DriftingspaceLambda<S: StateMachine + 'static> {
    state_store: DynamoDbStateStore<S>,
    api_gateway_client: ApiGatewayManagementApiClient,
}

impl<S: StateMachine + 'static> DriftingspaceLambda<S> {
    pub fn new(table_name: String, region: String) -> Self {
        Self {
            state_store: DynamoDbStateStore::new(table_name),
            api_gateway_client: ApiGatewayManagementApiClient::new(&region),
        }
    }

    pub fn handler(&mut self, event: LambdaEvent) -> Result<ApiGatewayV2HttpResponse, String> {
        match event.event {
            LambdaEventType::WebSocketMessage(msg) => self.handle_message(msg),
            LambdaEventType::WebSocketConnect(conn) => self.handle_connect(conn),
            LambdaEventType::Http(http) => self.handle_http(http),
        }
    }

    fn handle_message(&mut self, event: WebSocketMessageEvent) -> Result<ApiGatewayV2HttpResponse, String> {
        let connection_id = event.connection_id;
        let body = event.body.ok_or("Missing body")?;

        // Load connection state
        let mut conn_state: ConnectionState<S> = self
            .state_store
            .get_state(&connection_id)?
            .unwrap_or_else(|| ConnectionState::new(S::new()));

        // Parse message
        let message: MessageToServer<S> = serde_json::from_str(&body)?;

        // Process with Valtron
        let mut executor = Executor::new();
        let task = ProcessTransitionTask::new(conn_state.state.clone(), message);
        executor.spawn(task);

        // Run executor
        let result = executor.run_until_complete();

        // Update state and save
        conn_state.state = result.state;
        conn_state.version = result.version;
        self.state_store.save_state(&connection_id, &conn_state)?;

        // Broadcast to other connections if needed
        if let Some(broadcast) = result.broadcast {
            self.broadcast_to_connections(&broadcast)?;
        }

        Ok(ApiGatewayV2HttpResponse::ok(serde_json::to_string(&result.response)?))
    }

    fn handle_connect(&mut self, event: WebSocketConnectEvent) -> Result<ApiGatewayV2HttpResponse, String> {
        let connection_id = event.connection_id;

        // Initialize new connection state
        let conn_state = ConnectionState::new(S::new());
        self.state_store.save_state(&connection_id, &conn_state)?;

        Ok(ApiGatewayV2HttpResponse::ok(serde_json::json!({
            "status": "connected",
            "connection_id": connection_id
        }).to_string()))
    }

    fn handle_http(&mut self, event: ApiGatewayV2HttpRequestEvent) -> Result<ApiGatewayV2HttpResponse, String> {
        // HTTP fallback for non-WebSocket clients
        // Similar to handle_message but without connection tracking
        todo!("Implement HTTP handler")
    }

    fn broadcast_to_connections(&self, message: &str) -> Result<(), String> {
        // Use API Gateway Management API to send to all connections
        self.api_gateway_client.broadcast(message)
    }
}
```

### Task Implementation

```rust
pub struct ProcessTransitionTask<S: StateMachine> {
    state: S,
    message: MessageToServer<S>,
    result: Option<ProcessResult<S>>,
}

pub struct ProcessResult<S: StateMachine> {
    pub state: S,
    pub version: StateVersionNumber,
    pub response: MessageToClient<S>,
    pub broadcast: Option<MessageToClient<S>>,
}

impl<S: StateMachine> ProcessTransitionTask<S> {
    pub fn new(state: S, message: MessageToServer<S>) -> Self {
        Self {
            state,
            message,
            result: None,
        }
    }
}

impl<S: StateMachine> TaskIterator for ProcessTransitionTask<S> {
    type Ready = ProcessResult<S>;
    type Pending = ();
    type Spawner = ();

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending>> {
        if self.result.is_some() {
            return None; // Already complete
        }

        let (response, broadcast, new_state, version) = match &self.message {
            MessageToServer::DoTransition { transition_number, transition } => {
                match self.state.apply(transition) {
                    Ok(state) => {
                        let version = StateVersionNumber(
                            // In real implementation, track version
                            1
                        );
                        (
                            MessageToClient::ConfirmTransition {
                                transition_number: *transition_number,
                                version,
                            },
                            Some(MessageToClient::PeerTransition {
                                transition: transition.clone(),
                                version,
                            }),
                            state,
                            version,
                        )
                    }
                    Err(conflict) => {
                        (
                            MessageToClient::Conflict {
                                transition_number: *transition_number,
                                conflict,
                            },
                            None,
                            self.state.clone(),
                            StateVersionNumber(0),
                        )
                    }
                }
            }
            MessageToServer::RequestState => {
                (
                    MessageToClient::SetState {
                        state: self.state.clone(),
                        version: StateVersionNumber(0),
                    },
                    None,
                    self.state.clone(),
                    StateVersionNumber(0),
                )
            }
        };

        let result = ProcessResult {
            state: new_state,
            version,
            response,
            broadcast,
        };

        self.result = Some(result.clone());
        Some(TaskStatus::Ready(result))
    }
}
```

---

## 8. Deployment Configuration

### Cargo.toml

```toml
[package]
name = "driftingspace-lambda"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
valtron = { path = "/home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron" }
aper = { path = "/home/darkvoid/Boxxed/@formulas/src.rust/src.driftingspace/aper/aper" }
aper-stateroom = { path = "/home/darkvoid/Boxxed/@formulas/src.rust/src.driftingspace/aper/aper-stateroom" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
ureq = "2.7"  # Sync HTTP client
```

### serverless.yml

```yaml
service: driftingspace

provider:
  name: aws
  runtime: provided.al2
  region: us-east-1
  memorySize: 256
  timeout: 30
  environment:
    STATE_TABLE: ${self:service}-state-${opt:stage, 'dev'}
    API_GATEWAY_REGION: ${self:provider.region}

functions:
  websocket:
    handler: driftingspace-lambda
    events:
      - websocket:
          route: $connect
      - websocket:
          route: $default
      - websocket:
          route: $disconnect

resources:
  Resources:
    StateTable:
      Type: AWS::DynamoDB::Table
      Properties:
        TableName: ${self:provider.environment.STATE_TABLE}
        AttributeDefinitions:
          - AttributeName: connection_id
            AttributeType: S
        KeySchema:
          - AttributeName: connection_id
            KeyType: HASH
        BillingMode: PAY_PER_REQUEST
```

### build.rs

```rust
use std::process::Command;

fn main() {
    // Build for Lambda target
    Command::new("cargo")
        .args(&["build", "--release", "--target", "x86_64-unknown-linux-gnu"])
        .status()
        .unwrap();

    // Copy binary to expected location
    std::fs::copy(
        "target/x86_64-unknown-linux-gnu/release/driftingspace-lambda",
        "bootstrap",
    ).unwrap();
}
```

---

## Summary

| Component | Valtron Approach |
|-----------|-----------------|
| Executor | Single-threaded, no tokio |
| Task Pattern | TaskIterator instead of async |
| State Store | DynamoDB (sync access) |
| WebSocket | API Gateway WebSocket API |
| Deployment | serverless.yml + Lambda |
| Cold Start | Minimized (no async runtime) |

---

## Document History

| Date | Change |
|------|--------|
| 2026-03-27 | Initial Valtron integration guide created |

---

*This exploration is a living document. Revisit sections as concepts become clearer through implementation.*
