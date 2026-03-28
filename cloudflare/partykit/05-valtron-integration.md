---
title: "Valtron Integration: Lambda Deployment for Multiplayer Backend"
subtitle: "Deploy multiplayer backend to AWS Lambda using valtron executor - No async/tokio patterns"
based_on: "Valtron executor, ewe_platform backends, PartyServer patterns"
---

# Valtron Integration: Lambda Deployment for Multiplayer Backend

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Lambda Runtime Setup](#2-lambda-runtime-setup)
3. [HTTP API Compatibility](#3-http-api-compatibility)
4. [WebSocket Handling](#4-websocket-handling)
5. [State Management](#5-state-management)
6. [Deployment Guide](#6-deployment-guide)
7. [Production Considerations](#7-production-considerations)

---

## 1. Architecture Overview

### 1.1 Why Lambda for Multiplayer?

| Aspect | Cloudflare Workers | AWS Lambda |
|--------|-------------------|------------|
| **Cold Start** | ~50ms | ~100-500ms |
| **Max Duration** | 15 minutes | 15 minutes |
| **Memory** | Up to 128MB | Up to 10GB |
| **State** | Durable Objects | External (DynamoDB, Redis) |
| **WebSocket** | Native | API Gateway |
| **Pricing** | Per-request + CPU | Per-request + duration |

**When to use Lambda:**
- Existing AWS infrastructure
- Need for larger memory/CPU
- Integration with AWS services
- Compliance requirements

### 1.2 Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                     AWS Infrastructure                           │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                   API Gateway                             │   │
│  │                                                            │   │
│  │  HTTP Endpoints           WebSocket Routes                │   │
│  │  - POST /connect          - $connect                       │   │
│  │  - POST /message          - $default                       │   │
│  │  - GET  /rooms            - $disconnect                    │   │
│  └────────────┬────────────────────┬──────────────────────────┘   │
│               │                    │                               │
│               │                    │                               │
│  ┌────────────▼────────────────────▼──────────────────────────┐   │
│  │                   Lambda Function                           │   │
│  │                                                             │   │
│  │  ┌─────────────────────────────────────────────────────┐   │   │
│  │  │              Valtron Executor                        │   │   │
│  │  │                                                       │   │   │
│  │  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐   │   │   │
│  │  │  │   Connect   │  │   Message   │  │  Disconnect │   │   │   │
│  │  │  │    Task     │  │    Task     │  │    Task     │   │   │   │
│  │  │  └─────────────┘  └─────────────┘  └─────────────┘   │   │   │
│  │  └─────────────────────────────────────────────────────┘   │   │
│  │                                                             │   │
│  │  ┌─────────────────────────────────────────────────────┐   │   │
│  │  │              Room State Manager                      │   │   │
│  │  └─────────────────────────────────────────────────────┘   │   │
│  └────────────┬────────────────────────────────────────────┘   │
│               │                                                 │
│  ┌────────────▼────────────────────────────────────────────┐   │
│  │                   Data Layer                              │   │
│  │                                                            │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐       │   │
│  │  │  DynamoDB   │  │   Elasti    │  │    S3       │       │   │
│  │  │  (State)    │  │  Cache      │  │  (Archives) │       │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘       │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

### 1.3 Valtron Executor Pattern

```rust
// No async/await, no tokio
// Uses TaskIterator pattern for async-like behavior

use foundation_core::valtron::{TaskIterator, TaskStatus, NoSpawner};

// Instead of:
// async fn handle_message(conn: Connection, msg: String) -> Result<()> { ... }

// Use:
struct HandleMessageTask {
    connection_id: String,
    message: String,
    step: usize,
}

impl TaskIterator for HandleMessageTask {
    type Ready = Result<(), Error>;
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        match self.step {
            0 => {
                // Validate message
                self.step = 1;
                Some(TaskStatus::Pending(()))
            }
            1 => {
                // Store in database
                self.step = 2;
                Some(TaskStatus::Pending(()))
            }
            2 => {
                // Broadcast to others
                self.step = 3;
                Some(TaskStatus::Ready(Ok(())))
            }
            _ => None,
        }
    }
}
```

---

## 2. Lambda Runtime Setup

### 2.1 Project Structure

```
lambda-multiplayer/
├── Cargo.toml
├── src/
│   ├── main.rs                 # Lambda entry point
│   ├── handler.rs              # Request handlers
│   ├── valtron_tasks/
│   │   ├── mod.rs              # Task module
│   │   ├── connect.rs          # Connect task
│   │   ├── message.rs          # Message task
│   │   └── disconnect.rs       # Disconnect task
│   ├── state/
│   │   ├── mod.rs              # State management
│   │   ├── room.rs             # Room state
│   │   └── connection.rs       # Connection state
│   ├── storage/
│   │   ├── mod.rs              # Storage module
│   │   ├── dynamodb.rs         # DynamoDB client
│   │   └── redis.rs            # Redis client
│   └── types/
│       ├── mod.rs              # Type definitions
│       └── messages.rs         # Message types
├── template.yaml               # SAM template
└── tests/
    └── integration.rs
```

### 2.2 Cargo Configuration

```toml
[package]
name = "lambda-multiplayer"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "bootstrap"
path = "src/main.rs"

[dependencies]
# AWS Lambda
lambda_runtime = "0.11"
aws-config = "1.5"
aws-sdk-dynamodb = "1.47"
aws-sdk-apigatewaymanagementapi = "1.48"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Valtron executor
foundation_core = { path = "../ewe_platform/backends/foundation_core" }

# Utilities
uuid = { version = "1.0", features = ["v4"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json"] }
thiserror = "1.0"
chrono = "0.4"

# Redis for presence
redis = { version = "0.25", features = ["tokio-comp"] }

# For HTTP client
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls"] }

[profile.release]
opt-level = 3
lto = true
strip = true
```

### 2.3 Lambda Entry Point

```rust
// src/main.rs
use lambda_runtime::{service_fn, Error, LambdaEvent};
use serde_json::{json, Value};
use tracing::{info, error};
use foundation_core::valtron::single::{initialize, run_until_complete};

mod handler;
mod valtron_tasks;
mod state;
mod storage;
mod types;

use handler::LambdaHandler;
use storage::StorageClient;

struct AppState {
    storage: StorageClient,
    handler: LambdaHandler,
}

impl AppState {
    async fn new() -> Result<Self, Error> {
        let config = aws_config::load_from_env().await;
        let storage = StorageClient::new(&config).await?;
        let handler = LambdaHandler::new(storage.clone());

        Ok(Self { storage, handler })
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .json()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting Lambda multiplayer backend");

    // Initialize valtron executor
    initialize(0);

    // Create shared state
    let state = std::sync::Arc::new(AppState::new().await?);

    // Define handler
    let func = service_fn({
        let state = state.clone();
        move |event: LambdaEvent<Value>| {
            let state = state.clone();
            async move {
                let response = state.handler.handle(event.payload).await?;
                Ok::<Value, Error>(response)
            }
        }
    });

    // Run Lambda runtime
    lambda_runtime::run(func).await?;

    Ok(())
}
```

### 2.4 SAM Template

```yaml
# template.yaml
AWSTemplateFormatVersion: '2010-09-09'
Transform: AWS::Serverless-2016-10-31

Resources:
  MultiplayerApi:
    Type: AWS::Serverless::Api
    Properties:
      StageName: prod
      Cors:
        AllowOrigin: "'*'"
        AllowMethods: "'GET,POST,OPTIONS'"
        AllowHeaders: "'Content-Type,Authorization'"

  MultiplayerWebSocket:
    Type: AWS::ApiGatewayV2::Api
    Properties:
      Name: multiplayer-ws
      ProtocolType: WEBSOCKET
      RouteSelectionExpression: "$request.body.action"

  ConnectRoute:
    Type: AWS::ApiGatewayV2::Route
    Properties:
      ApiId: !Ref MultiplayerWebSocket
      RouteKey: $connect
      AuthorizationType: NONE
      Target: !Join
        - '/'
        - - 'integrations'
          - !Ref ConnectIntegration

  DefaultRoute:
    Type: AWS::ApiGatewayV2::Route
    Properties:
      ApiId: !Ref MultiplayerWebSocket
      RouteKey: $default
      Target: !Join
        - '/'
        - - 'integrations'
          - !Ref DefaultIntegration

  DisconnectRoute:
    Type: AWS::ApiGatewayV2::Route
    Properties:
      ApiId: !Ref MultiplayerWebSocket
      RouteKey: $disconnect
      Target: !Join
        - '/'
        - 'integrations'
          - !Ref DisconnectIntegration

  ConnectIntegration:
    Type: AWS::ApiGatewayV2::Integration
    Properties:
      ApiId: !Ref MultiplayerWebSocket
      IntegrationType: AWS_PROXY
      IntegrationUri: !GetAtt MultiplayerFunction.Arn
      IntegrationMethod: POST

  DefaultIntegration:
    Type: AWS::ApiGatewayV2::Integration
    Properties:
      ApiId: !Ref MultiplayerWebSocket
      IntegrationType: AWS_PROXY
      IntegrationUri: !GetAtt MultiplayerFunction.Arn
      IntegrationMethod: POST

  DisconnectIntegration:
    Type: AWS::ApiGatewayV2::Integration
    Properties:
      ApiId: !Ref MultiplayerWebSocket
      IntegrationType: AWS_PROXY
      IntegrationUri: !GetAtt MultiplayerFunction.Arn
      IntegrationMethod: POST

  MultiplayerFunction:
    Type: AWS::Serverless::Function
    Properties:
      CodeUri: target/lambda/bootstrap/
      Handler: bootstrap
      Runtime: provided.al2
      MemorySize: 256
      Timeout: 30
      Environment:
        Variables:
          ROOMS_TABLE: !Ref RoomsTable
          CONNECTIONS_TABLE: !Ref ConnectionsTable
          PRESENCE_CACHE: !Ref PresenceCache
      Policies:
        - DynamoDBCrudPolicy:
            TableName: !Ref RoomsTable
        - DynamoDBCrudPolicy:
            TableName: !Ref ConnectionsTable
        - Statement:
            - Effect: Allow
              Action:
                - execute-api:ManageConnections
              Resource: !Sub "${MultiplayerWebSocket}/*"
      Events:
        HttpApi:
          Type: Api
          Properties:
            RestApiId: !Ref MultiplayerApi
            Path: /{proxy+}
            Method: ANY
        WebSocket:
          Type: WebSocketApi
          Properties:
            Api: !Ref MultiplayerWebSocket

  RoomsTable:
    Type: AWS::DynamoDB::Table
    Properties:
      TableName: multiplayer-rooms
      BillingMode: PAY_PER_REQUEST
      AttributeDefinitions:
        - AttributeName: room_id
          AttributeType: S
      KeySchema:
        - AttributeName: room_id
          KeyType: HASH

  ConnectionsTable:
    Type: AWS::DynamoDB::Table
    Properties:
      TableName: multiplayer-connections
      BillingMode: PAY_PER_REQUEST
      AttributeDefinitions:
        - AttributeName: connection_id
          AttributeType: S
        - AttributeName: room_id
          AttributeType: S
      KeySchema:
        - AttributeName: connection_id
          KeyType: HASH
      GlobalSecondaryIndexes:
        - IndexName: room-index
          KeySchema:
            - AttributeName: room_id
              KeyType: HASH
          Projection:
            ProjectionType: ALL

  PresenceCache:
    Type: AWS::ElastiCache::CacheCluster
    Properties:
      CacheClusterId: multiplayer-presence
      Engine: redis
      CacheNodeType: cache.t3.micro
      NumCacheNodes: 1

Outputs:
  ApiEndpoint:
    Description: HTTP API Endpoint
    Value: !Sub "https://${MultiplayerApi}.execute-api.${AWS::Region}.amazonaws.com/prod"

  WebSocketEndpoint:
    Description: WebSocket API Endpoint
    Value: !Sub "${MultiplayerWebSocket.execute-api.${AWS::Region}.amazonaws.com/prod"
```

---

## 3. HTTP API Compatibility

### 3.1 Request/Response Types

```rust
// src/types/mod.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiGatewayRequest {
    pub resource: String,
    pub path: String,
    pub http_method: String,
    pub headers: std::collections::HashMap<String, String>,
    pub body: Option<String>,
    pub is_base64_encoded: bool,
    #[serde(default)]
    pub query_string_parameters: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiGatewayResponse {
    pub status_code: u16,
    pub headers: std::collections::HashMap<String, String>,
    pub body: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_base64_encoded: Option<bool>,
}

impl ApiGatewayResponse {
    pub fn ok(body: &str) -> Self {
        Self {
            status_code: 200,
            headers: [("Content-Type".to_string(), "application/json".to_string())]
                .into_iter()
                .collect(),
            body: body.to_string(),
            is_base64_encoded: None,
        }
    }

    pub fn error(status: u16, message: &str) -> Self {
        let body = serde_json::json!({ "error": message }).to_string();
        Self {
            status_code: status,
            headers: [("Content-Type".to_string(), "application/json".to_string())]
                .into_iter()
                .collect(),
            body,
            is_base64_encoded: None,
        }
    }

    pub fn with_cors(mut self) -> Self {
        self.headers.insert("Access-Control-Allow-Origin".to_string(), "*".to_string());
        self.headers.insert("Access-Control-Allow-Methods".to_string(), "GET, POST, HEAD, OPTIONS".to_string());
        self.headers.insert("Access-Control-Allow-Headers".to_string(), "Content-Type, Authorization".to_string());
        self
    }
}
```

### 3.2 HTTP Handlers

```rust
// src/handler.rs
use crate::types::{ApiGatewayRequest, ApiGatewayResponse};
use crate::storage::StorageClient;
use crate::valtron_tasks::{ConnectTask, MessageTask};
use foundation_core::valtron::single::{spawn, run_until_complete};
use serde_json::{json, Value};

pub struct LambdaHandler {
    storage: StorageClient,
}

impl LambdaHandler {
    pub fn new(storage: StorageClient) -> Self {
        Self { storage }
    }

    pub async fn handle(&self, event: Value) -> Result<Value, Box<dyn std::error::Error>> {
        // Check if WebSocket event
        if let Some(request_context) = event.get("requestContext") {
            if let Some(route_key) = request_context.get("routeKey") {
                return self.handle_websocket(event).await;
            }
        }

        // HTTP API event
        self.handle_http(event).await
    }

    async fn handle_http(&self, event: Value) -> Result<Value, Box<dyn std::error::Error>> {
        let req: ApiGatewayRequest = serde_json::from_value(event.clone())?;

        let response = match (req.http_method.as_str(), req.path.as_str()) {
            ("POST", path) if path.ends_with("/connect") => self.connect_room(req).await?,
            ("POST", path) if path.ends_with("/message") => self.send_message(req).await?,
            ("GET", path) if path.ends_with("/rooms") => self.list_rooms(req).await?,
            ("OPTIONS", _) => self.cors_preflight(),
            _ => ApiGatewayResponse::error(404, "Not found"),
        };

        Ok(serde_json::to_value(response.with_cors())?)
    }

    async fn connect_room(&self, req: ApiGatewayRequest) -> Result<ApiGatewayResponse, Box<dyn std::error::Error>> {
        #[derive(Deserialize)]
        struct ConnectRequest {
            room_id: String,
            user_id: String,
        }

        let body: ConnectRequest = serde_json::from_str(&req.body.unwrap_or_default())?;

        // Create connection using valtron task
        let task = ConnectTask::new(
            body.room_id,
            body.user_id,
            self.storage.clone(),
        );

        spawn()
            .with_task(task)
            .schedule()?;

        run_until_complete();

        Ok(ApiGatewayResponse::ok(&json!({
            "success": true,
            "message": "Connected to room"
        }).to_string()))
    }

    async fn send_message(&self, req: ApiGatewayRequest) -> Result<ApiGatewayResponse, Box<dyn std::error::Error>> {
        #[derive(Deserialize)]
        struct MessageRequest {
            room_id: String,
            connection_id: String,
            content: String,
        }

        let body: MessageRequest = serde_json::from_str(&req.body.unwrap_or_default())?;

        // Process message using valtron task
        let task = MessageTask::new(
            body.room_id,
            body.connection_id,
            body.content,
            self.storage.clone(),
        );

        spawn()
            .with_task(task)
            .schedule()?;

        run_until_complete();

        Ok(ApiGatewayResponse::ok(&json!({
            "success": true,
            "message": "Message sent"
        }).to_string()))
    }

    async fn list_rooms(&self, _req: ApiGatewayRequest) -> Result<ApiGatewayResponse, Box<dyn std::error::Error>> {
        let rooms = self.storage.list_rooms().await?;

        Ok(ApiGatewayResponse::ok(&json!(rooms).to_string()))
    }

    fn cors_preflight(&self) -> ApiGatewayResponse {
        ApiGatewayResponse::ok("")
            .with_cors()
    }
}
```

---

## 4. WebSocket Handling

### 4.1 WebSocket Event Types

```rust
// src/types/websocket.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketEvent {
    pub request_context: RequestContext,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub is_base64_encoded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestContext {
    pub route_key: String,
    pub message_id: Option<String>,
    pub event_type: String,
    pub extended_request_id: String,
    pub request_id: String,
    pub route_key: String,
    pub api_id: String,
    pub domain_name: String,
    pub stage: String,
    pub connection_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum WebSocketMessage {
    #[serde(rename = "join")]
    Join { room_id: String },

    #[serde(rename = "message")]
    ChatMessage { content: String },

    #[serde(rename = "typing")]
    Typing { is_typing: bool },

    #[serde(rename = "heartbeat")]
    Heartbeat { timestamp: u64 },

    #[serde(rename = "leave")]
    Leave { room_id: String },
}
```

### 4.2 WebSocket Handler

```rust
// src/handler/websocket.rs
use crate::types::{WebSocketEvent, WebSocketMessage, ApiGatewayResponse};
use crate::storage::StorageClient;
use crate::valtron_tasks::{ConnectTask, MessageTask, DisconnectTask};
use foundation_core::valtron::single::spawn;
use aws_sdk_apigatewaymanagementapi::Client as ApiGatewayClient;

pub struct WebSocketHandler {
    storage: StorageClient,
    api_client: ApiGatewayClient,
}

impl WebSocketHandler {
    pub fn new(storage: StorageClient, api_client: ApiGatewayClient) -> Self {
        Self { storage, api_client }
    }

    pub async fn handle(&self, event: WebSocketEvent) -> Result<ApiGatewayResponse, Box<dyn std::error::Error>> {
        match event.request_context.event_type.as_str() {
            "CONNECT" => self.on_connect(event).await,
            "MESSAGE" => self.on_message(event).await,
            "DISCONNECT" => self.on_disconnect(event).await,
            _ => Ok(ApiGatewayResponse::error(400, "Unknown event type")),
        }
    }

    async fn on_connect(&self, event: WebSocketEvent) -> Result<ApiGatewayResponse, Box<dyn std::error::Error>> {
        let connection_id = event.request_context.connection_id
            .ok_or("Missing connection_id")?;

        // Store connection in DynamoDB
        self.storage.save_connection(&connection_id, "connected").await?;

        // Schedule connect task
        let task = ConnectTask::new(
            "default".to_string(),  // Default room
            connection_id,
            self.storage.clone(),
        );

        spawn().with_task(task).schedule()?;

        Ok(ApiGatewayResponse::ok("{}"))
    }

    async fn on_message(&self, event: WebSocketEvent) -> Result<ApiGatewayResponse, Box<dyn std::error::Error>> {
        let connection_id = event.request_context.connection_id
            .ok_or("Missing connection_id")?;

        let body = event.body.ok_or("Missing body")?;
        let message: WebSocketMessage = serde_json::from_str(&body)?;

        match message {
            WebSocketMessage::Join { room_id } => {
                self.handle_join(connection_id, room_id).await?;
            }
            WebSocketMessage::ChatMessage { content } => {
                self.handle_chat_message(connection_id, content).await?;
            }
            WebSocketMessage::Typing { is_typing } => {
                self.handle_typing(connection_id, is_typing).await?;
            }
            WebSocketMessage::Heartbeat { timestamp } => {
                self.handle_heartbeat(connection_id, timestamp).await?;
            }
            WebSocketMessage::Leave { room_id } => {
                self.handle_leave(connection_id, room_id).await?;
            }
        }

        Ok(ApiGatewayResponse::ok("{}"))
    }

    async fn on_disconnect(&self, event: WebSocketEvent) -> Result<ApiGatewayResponse, Box<dyn std::error::Error>> {
        let connection_id = event.request_context.connection_id
            .ok_or("Missing connection_id")?;

        // Schedule disconnect task
        let task = DisconnectTask::new(
            connection_id,
            self.storage.clone(),
        );

        spawn().with_task(task).schedule()?;

        // Remove from DynamoDB
        self.storage.delete_connection(&connection_id).await?;

        Ok(ApiGatewayResponse::ok("{}"))
    }

    async fn handle_join(&self, connection_id: String, room_id: String) -> Result<(), Box<dyn std::error::Error>> {
        // Update connection's room
        self.storage.update_connection_room(&connection_id, &room_id).await?;

        // Notify room members
        let room_connections = self.storage.get_room_connections(&room_id).await?;

        for conn_id in room_connections {
            if conn_id != connection_id {
                self.send_to_connection(&conn_id, &serde_json::json!({
                    "type": "user_joined",
                    "connection_id": connection_id,
                    "room_id": room_id
                }).to_string()).await?;
            }
        }

        Ok(())
    }

    async fn handle_chat_message(&self, connection_id: String, content: String) -> Result<(), Box<dyn std::error::Error>> {
        // Get connection's room
        let connection = self.storage.get_connection(&connection_id).await?;
        let room_id = connection.room_id.ok_or("Not in a room")?;

        // Store message
        let message_id = uuid::Uuid::new_v4().to_string();
        self.storage.save_message(&message_id, &room_id, &connection_id, &content).await?;

        // Broadcast to room
        let room_connections = self.storage.get_room_connections(&room_id).await?;

        for conn_id in room_connections {
            self.send_to_connection(&conn_id, &serde_json::json!({
                "type": "message",
                "id": message_id,
                "content": content,
                "sender_id": connection_id,
                "timestamp": chrono::Utc::now().timestamp_millis()
            }).to_string()).await?;
        }

        Ok(())
    }

    async fn send_to_connection(&self, connection_id: &str, message: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.api_client
            .post_to_connection()
            .connection_id(connection_id)
            .body(message)
            .send()
            .await?;
        Ok(())
    }
}
```

---

## 5. State Management

### 5.1 DynamoDB Storage Client

```rust
// src/storage/dynamodb.rs
use aws_sdk_dynamodb::{Client, types::AttributeValue};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct StorageClient {
    client: Client,
    rooms_table: String,
    connections_table: String,
}

impl StorageClient {
    pub fn new(config: &aws_sdk_dynamodb::Config) -> Self {
        let client = Client::new(config.clone());
        Self {
            client,
            rooms_table: std::env::var("ROOMS_TABLE").unwrap_or_default(),
            connections_table: std::env::var("CONNECTIONS_TABLE").unwrap_or_default(),
        }
    }

    pub async fn save_connection(&self, connection_id: &str, status: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.client
            .put_item()
            .table_name(&self.connections_table)
            .item("connection_id", AttributeValue::S(connection_id.to_string()))
            .item("status", AttributeValue::S(status.to_string()))
            .item("created_at", AttributeValue::N(chrono::Utc::now().timestamp_millis().to_string()))
            .send()
            .await?;
        Ok(())
    }

    pub async fn get_connection(&self, connection_id: &str) -> Result<ConnectionRecord, Box<dyn std::error::Error>> {
        let result = self.client
            .get_item()
            .table_name(&self.connections_table)
            .key("connection_id", AttributeValue::S(connection_id.to_string()))
            .send()
            .await?;

        let item = result.item.ok_or("Connection not found")?;
        Ok(ConnectionRecord::from_item(item)?)
    }

    pub async fn update_connection_room(&self, connection_id: &str, room_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.client
            .update_item()
            .table_name(&self.connections_table)
            .key("connection_id", AttributeValue::S(connection_id.to_string()))
            .update_expression("SET room_id = :room_id")
            .expression_attribute_values(":room_id", AttributeValue::S(room_id.to_string()))
            .send()
            .await?;
        Ok(())
    }

    pub async fn get_room_connections(&self, room_id: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let result = self.client
            .query()
            .table_name(&self.connections_table)
            .index_name("room-index")
            .key_condition_expression("room_id = :room_id")
            .expression_attribute_values(":room_id", AttributeValue::S(room_id.to_string()))
            .send()
            .await?;

        let connections = result.items
            .unwrap_or_default()
            .iter()
            .filter_map(|item| item.get("connection_id"))
            .filter_map(|v| match v {
                AttributeValue::S(s) => Some(s.clone()),
                _ => None,
            })
            .collect();

        Ok(connections)
    }

    pub async fn delete_connection(&self, connection_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.client
            .delete_item()
            .table_name(&self.connections_table)
            .key("connection_id", AttributeValue::S(connection_id.to_string()))
            .send()
            .await?;
        Ok(())
    }

    pub async fn save_message(&self, message_id: &str, room_id: &str, sender_id: &str, content: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.client
            .put_item()
            .table_name(&self.rooms_table)
            .item("pk", AttributeValue::S(format!("ROOM#{}", room_id)))
            .item("sk", AttributeValue::S(format!("MSG#{}", message_id)))
            .item("room_id", AttributeValue::S(room_id.to_string()))
            .item("sender_id", AttributeValue::S(sender_id.to_string()))
            .item("content", AttributeValue::S(content.to_string()))
            .item("created_at", AttributeValue::N(chrono::Utc::now().timestamp_millis().to_string()))
            .send()
            .await?;
        Ok(())
    }

    pub async fn list_rooms(&self) -> Result<Vec<RoomInfo>, Box<dyn std::error::Error>> {
        // Implementation for listing rooms
        Ok(vec![])
    }
}

#[derive(Debug, Clone)]
pub struct ConnectionRecord {
    pub connection_id: String,
    pub status: String,
    pub room_id: Option<String>,
    pub created_at: i64,
}

impl ConnectionRecord {
    pub fn from_item(item: HashMap<String, AttributeValue>) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            connection_id: item.get("connection_id")
                .and_then(|v| v.as_s().ok())
                .ok_or("Missing connection_id")?
                .clone(),
            status: item.get("status")
                .and_then(|v| v.as_s().ok())
                .ok_or("Missing status")?
                .clone(),
            room_id: item.get("room_id").and_then(|v| v.as_s().ok()).cloned(),
            created_at: item.get("created_at")
                .and_then(|v| v.as_n().ok())
                .ok_or("Missing created_at")?
                .parse()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct RoomInfo {
    pub room_id: String,
    pub connection_count: i32,
}
```

---

## 6. Deployment Guide

### 6.1 Build and Deploy

```bash
# Install cross-compilation target
rustup target add x86_64-unknown-linux-musl

# Install lambda build tools
cargo install cargo-lambda

# Build for Lambda
cargo lambda build --release --target x86_64-unknown-linux-musl

# Deploy with SAM
sam build
sam deploy --guided

# Or deploy with AWS CLI
aws cloudformation package \
  --template-file template.yaml \
  --s3-bucket my-deployment-bucket \
  --output-template-file packaged.yaml

aws cloudformation deploy \
  --template-file packaged.yaml \
  --stack-name lambda-multiplayer \
  --capabilities CAPABILITY_IAM
```

### 6.2 Environment Configuration

```bash
# Set environment variables
export AWS_REGION=us-east-1
export ROOMS_TABLE=multiplayer-rooms
export CONNECTIONS_TABLE=multiplayer-connections
export PRESENCE_CACHE=my-redis-cluster.xxx.use1.cache.amazonaws.com:6379
export JWT_SECRET=your-secret-key
```

### 6.3 Testing

```bash
# Test locally with lambda runtime
cargo lambda watch

# Invoke locally
cargo lambda invoke --data-file test-events/connect.json

# Test WebSocket connection
wscat -c wss://your-api-id.execute-api.us-east-1.amazonaws.com/prod

# Send message
{"action": "join", "room_id": "room1"}
{"action": "message", "content": "Hello!"}
```

---

## 7. Production Considerations

### 7.1 Cold Start Mitigation

```rust
// Use provisioned concurrency
// In SAM template:
MultiplayerFunction:
  Type: AWS::Serverless::Function
  Properties:
    ProvisionedConcurrencyConfig:
      ProvisionedConcurrentExecutions: 10

// Keep warm with scheduled pings
WarmupFunction:
  Type: AWS::Serverless::Function
  Properties:
    Handler: bootstrap
    Runtime: provided.al2
    Timeout: 5
    Events:
      Schedule:
        Type: Schedule
        Properties:
          Schedule: rate(1 minute)
```

### 7.2 Connection State Recovery

```rust
// On reconnect, recover state from DynamoDB
async fn recover_connection_state(&self, connection_id: &str) -> Result<ConnectionState, Error> {
    let connection = self.storage.get_connection(connection_id).await?;

    // Fetch room state
    if let Some(room_id) = &connection.room_id {
        let messages = self.storage.get_room_messages(room_id, 50).await?;
        let presence = self.storage.get_room_presence(room_id).await?;

        Ok(ConnectionState {
            room_id: Some(room_id.clone()),
            recent_messages: messages,
            presence,
        })
    } else {
        Ok(ConnectionState::default())
    }
}
```

### 7.3 Cost Optimization

```rust
// Batch DynamoDB operations
async fn batch_save_connections(&self, connections: Vec<ConnectionRecord>) -> Result<(), Error> {
    let mut request_items = HashMap::new();

    let items: Vec<_> = connections
        .into_iter()
        .map(|c| {
            let mut item = HashMap::new();
            item.insert("connection_id".to_string(), AttributeValue::S(c.connection_id));
            item.insert("status".to_string(), AttributeValue::S(c.status));
            item
        })
        .collect();

    request_items.insert(self.connections_table.clone(), items);

    self.client
        .batch_write_item()
        .set_request_items(Some(request_items))
        .send()
        .await?;

    Ok(())
}
```

---

## Document History

| Date | Change |
|------|--------|
| 2026-03-27 | Initial Valtron integration guide created |

---

*This exploration is a living document. Revisit sections as concepts become clearer through implementation.*
