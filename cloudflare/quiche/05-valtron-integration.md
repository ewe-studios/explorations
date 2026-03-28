---
title: "Valtron Integration: Serverless QUIC Proxy"
subtitle: "Lambda deployment for QUIC proxy without async/tokio"
---

# Valtron Integration: Serverless QUIC Proxy

## Introduction

This document describes how to deploy quiche-based QUIC/HTTP3 proxy on AWS Lambda using the valtron executor. We cover the TaskIterator pattern for QUIC event processing, UDP socket handling in Lambda, and connection state serialization.

## Table of Contents

1. [Lambda QUIC Architecture](#1-lambda-quic-architecture)
2. [TaskIterator for QUIC Events](#2-taskiterator-for-quic-events)
3. [UDP Socket Handling](#3-udp-socket-handling)
4. [Connection State Serialization](#4-connection-state-serialization)
5. [Lambda Deployment](#5-lambda-deployment)
6. [API Gateway Integration](#6-api-gateway-integration)
7. [Cost Optimization](#7-cost-optimization)

---

## 1. Lambda QUIC Architecture

### 1.1 Why QUIC on Lambda?

```
Traditional QUIC Server:
┌─────────────────────────────┐
│   Long-running process      │
│   ┌─────────────────────┐   │
│   │ Connection State    │   │
│   │ (in memory)         │   │
│   └─────────────────────┘   │
│   Persistent UDP socket     │
└─────────────────────────────┘

Problem: Expensive idle connections

Serverless QUIC Proxy:
┌─────────────────────────────┐
│   Lambda Function           │
│   ┌─────────────────────┐   │
│   │ QUIC Connection     │   │
│   │ (per invocation)    │   │
│   └─────────────────────┘   │
│   Stateless (DDB for state) │
└─────────────────────────────┘

Benefit: Pay only for active processing
```

### 1.2 Architecture Overview

```
┌──────────────────────────────────────────────────────────────┐
│                         Client                                │
│                    (QUIC/HTTP3)                               │
└─────────────────────────┬────────────────────────────────────┘
                          │ UDP:443
                          ▼
┌──────────────────────────────────────────────────────────────┐
│                    AWS Lambda (valtron)                       │
│  ┌────────────────────────────────────────────────────────┐   │
│  │  QuicProxyTask (TaskIterator)                          │   │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐ │   │
│  │  │   Receiving  │─►│  Processing  │─►│   Sending    │ │   │
│  │  └──────────────┘  └──────────────┘  └──────────────┘ │   │
│  └────────────────────────────────────────────────────────┘   │
│                            │                                   │
│                            ▼                                   │
│                 ┌─────────────────────┐                        │
│                 │ Connection State    │                        │
│                 │ (DynamoDB)          │                        │
│                 └─────────────────────┘                        │
└──────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌──────────────────────────────────────────────────────────────┐
│                    Origin Server                              │
│                 (HTTP/1.1, HTTP/2, HTTP/3)                    │
└──────────────────────────────────────────────────────────────┘
```

### 1.3 No Async/Tokio Constraint

Lambda doesn't support long-running async loops. We use valtron's TaskIterator:

```rust
// NOT this (async/tokio - doesn't work in Lambda):
async fn quic_event_loop(conn: Connection, socket: UdpSocket) {
    loop {
        let (len, from) = socket.recv_from(&mut buf).await?;
        conn.recv(&mut buf[..len], info)?;
        // ...
    }
}

// Use valtron TaskIterator instead:
pub struct QuicProxyTask {
    conn: Connection,
    state: QuicTaskState,
}

impl TaskIterator for QuicProxyTask {
    type Ready = QuicResult;
    type Pending = ();

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending>> {
        // State machine drives execution
        match self.state {
            QuicTaskState::Receive => { /* ... */ }
            QuicTaskState::Process => { /* ... */ }
            QuicTaskState::Send => { /* ... */ }
        }
    }
}
```

---

## 2. TaskIterator for QUIC Events

### 2.1 QUIC Task State Machine

```rust
use valtron::{TaskIterator, TaskStatus, NoSpawner};

/// QUIC connection processing task
pub struct QuicProxyTask {
    /// QUIC connection
    conn: Connection,
    /// UDP socket (Lambda-provided)
    socket: LambdaUdpSocket,
    /// Receive buffer
    recv_buf: Vec<u8>,
    /// Send buffer
    send_buf: Vec<u8>,
    /// Current task state
    state: QuicTaskState,
    /// Peer address from event
    peer: SocketAddr,
    /// Connection ID for state lookup
    conn_id: ConnectionId,
}

#[derive(Clone, Copy, PartialEq)]
pub enum QuicTaskState {
    /// Initial state - parse incoming packet
    Init,
    /// Process received packet through QUIC
    Processing,
    /// Generate response packets
    Sending,
    /// Wait for timeout (yield to executor)
    Waiting,
    /// Serialize state and exit
    Persisting,
    /// Task complete
    Done,
}

impl TaskIterator for QuicProxyTask {
    type Ready = QuicTaskResult;
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        match self.state {
            QuicTaskState::Init => self.handle_init(),
            QuicTaskState::Processing => self.handle_processing(),
            QuicTaskState::Sending => self.handle_sending(),
            QuicTaskState::Waiting => self.handle_waiting(),
            QuicTaskState::Persisting => self.handle_persisting(),
            QuicTaskState::Done => None,
        }
    }
}
```

### 2.2 Init State - Parse Event

```rust
impl QuicProxyTask {
    fn handle_init(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending>> {
        // Lambda invocation contains UDP datagram
        let event: LambdaUdpEvent = self.socket.get_event()?;

        self.peer = event.source_ip_port;
        self.conn_id = extract_connection_id(&event.payload)?;

        // Load or create connection state
        match load_connection_state(&self.conn_id) {
            Some(state) => {
                self.conn = deserialize_connection(state);
            }
            None => {
                // New connection - create
                self.conn = Connection::accept(
                    &self.conn_id,
                    None,
                    event.dest_ip_port,
                    self.peer,
                    &CONFIG,
                ).ok()?;
            }
        }

        // Queue packet for processing
        self.recv_buf = event.payload;
        self.state = QuicTaskState::Processing;

        Some(TaskStatus::Pending(()))
    }
}
```

### 2.3 Processing State - QUIC recv()

```rust
impl QuicProxyTask {
    fn handle_processing(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending>> {
        let recv_info = RecvInfo {
            from: self.peer,
            to: self.socket.local_addr(),
        };

        match self.conn.recv(&mut self.recv_buf, recv_info) {
            Ok(_) => {
                // Packet processed successfully
                self.state = QuicTaskState::Sending;
                Some(TaskStatus::Pending(()))
            }
            Err(quiche::Error::Done) => {
                // No more processing needed
                self.state = QuicTaskState::Sending;
                Some(TaskStatus::Pending(()))
            }
            Err(e) => {
                // Connection error - close
                self.state = QuicTaskState::Persisting;
                Some(TaskStatus::Ready(QuicTaskResult::Error(e)))
            }
        }
    }
}
```

### 2.4 Sending State - QUIC send()

```rust
impl QuicProxyTask {
    fn handle_sending(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending>> {
        // Generate all pending packets
        let mut packets = Vec::new();

        loop {
            match self.conn.send(&mut self.send_buf) {
                Ok((len, send_info)) => {
                    packets.push((self.send_buf[..len].to_vec(), send_info));
                    // Continue to coalesce more packets
                }
                Err(quiche::Error::Done) => break,
                Err(e) => {
                    self.state = QuicTaskState::Persisting;
                    return Some(TaskStatus::Ready(QuicTaskResult::Error(e)));
                }
            }
        }

        if packets.is_empty() {
            // No packets to send - check if connection needs to persist
            if self.conn.should_close() {
                self.state = QuicTaskState::Done;
                Some(TaskStatus::Ready(QuicTaskResult::Closed))
            } else if let Some(timeout) = self.conn.timeout() {
                // Schedule wake-up for timeout
                self.wakeup_time = Some(Instant::now() + timeout);
                self.state = QuicTaskState::Persisting;
                Some(TaskStatus::Ready(QuicTaskResult::Waiting(timeout)))
            } else {
                self.state = QuicTaskState::Persisting;
                Some(TaskStatus::Ready(QuicTaskResult::Idle))
            }
        } else {
            // Send packets via Lambda response
            self.state = QuicTaskState::Persisting;
            Some(TaskStatus::Ready(QuicTaskResult::Packets(packets)))
        }
    }
}
```

### 2.5 Persisting State - Save Connection

```rust
impl QuicProxyTask {
    fn handle_persisting(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending>> {
        // Serialize connection state for next invocation
        let state = serialize_connection(&self.conn);

        // Store in DynamoDB
        save_connection_state(&self.conn_id, &state)?;

        self.state = QuicTaskState::Done;
        Some(TaskStatus::Ready(QuicTaskResult::Persisted))
    }
}
```

---

## 3. UDP Socket Handling

### 3.1 Lambda UDP Event Structure

```rust
use serde::{Deserialize, Serialize};

/// Lambda UDP event (custom runtime)
#[derive(Debug, Deserialize, Serialize)]
pub struct LambdaUdpEvent {
    /// Source IP and port
    #[serde(with = "socketaddr_serde")]
    pub source_ip_port: SocketAddr,
    /// Destination IP and port
    #[serde(with = "socketaddr_serde")]
    pub dest_ip_port: SocketAddr,
    /// UDP payload (QUIC packet)
    #[serde(with = "base64_serde")]
    pub payload: Vec<u8>,
    /// Event timestamp
    pub timestamp_ms: u64,
}

/// Lambda UDP response
#[derive(Debug, Serialize)]
pub struct LambdaUdpResponse {
    /// Packets to send
    pub packets: Vec<UdpPacket>,
    /// Connection state update
    pub state: Option<ConnectionState>,
    /// Next wakeup time (for timer)
    pub wakeup_after_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct UdpPacket {
    /// Destination address
    #[serde(with = "socketaddr_serde")]
    pub to: SocketAddr,
    /// Packet data
    #[serde(with = "base64_serde")]
    pub data: Vec<u8>,
}
```

### 3.2 Lambda Runtime Integration

```rust
use valtron::executor::Executor;

pub struct LambdaQuicRuntime {
    executor: Executor,
}

impl LambdaQuicRuntime {
    pub fn new() -> Self {
        Self {
            executor: Executor::new(),
        }
    }

    /// Handle Lambda invocation
    pub fn handle_event(&mut self, event: LambdaUdpEvent) -> LambdaUdpResponse {
        // Create task for this QUIC event
        let task = QuicProxyTask::new(event);

        // Run task to completion
        let result = self.executor.run_task(task);

        // Build response
        let mut response = LambdaUdpResponse {
            packets: Vec::new(),
            state: None,
            wakeup_after_ms: None,
        };

        match result {
            QuicTaskResult::Packets(packets) => {
                response.packets = packets
                    .into_iter()
                    .map(|(data, info)| UdpPacket {
                        to: info.to,
                        data,
                    })
                    .collect();
            }
            QuicTaskResult::Waiting(timeout) => {
                response.wakeup_after_ms = Some(timeout.as_millis() as u64);
            }
            _ => {}
        }

        response
    }
}
```

### 3.3 Event Source Mapping

```rust
// Lambda function entry point (custom runtime)
#[tokio::main]  // Only for Lambda runtime, not QUIC processing
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut runtime = LambdaQuicRuntime::new();

    // Poll Lambda runtime API
    loop {
        let event: LambdaUdpEvent = poll_runtime_api().await?;

        let response = runtime.handle_event(event);

        send_runtime_response(&response).await?;
    }
}

// Alternative: Use valtron's Lambda adapter
fn main() -> Result<(), Box<dyn std::error::Error>> {
    valtron::lambda::run(|event: LambdaUdpEvent| {
        let mut task = QuicProxyTask::new(event);
        Ok(task.run_to_completion())
    })
}
```

---

## 4. Connection State Serialization

### 4.1 Serializable Connection State

```rust
use serde::{Deserialize, Serialize};

/// Serialized QUIC connection state
#[derive(Debug, Serialize, Deserialize)]
pub struct ConnectionState {
    /// Connection ID
    pub conn_id: Vec<u8>,
    /// Connection state (handshake, established, etc.)
    pub state: ConnectionStatus,
    /// TLS session ticket (for 0-RTT)
    pub tls_session: Option<Vec<u8>>,
    /// Stream states
    pub streams: Vec<StreamState>,
    /// Flow control state
    pub flow_control: FlowControlState,
    /// Recovery state (CC, RTT)
    pub recovery: RecoveryState,
    /// Peer address
    #[serde(with = "socketaddr_serde")]
    pub peer_addr: SocketAddr,
    /// Last activity timestamp
    pub last_activity_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ConnectionStatus {
    Handshake,
    Established,
    Closing,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StreamState {
    pub stream_id: u64,
    pub send_offset: u64,
    pub recv_offset: u64,
    pub send_state: StreamStateType,
    pub recv_state: StreamStateType,
    pub priority: StreamPriority,
}
```

### 4.2 Serialization Implementation

```rust
pub fn serialize_connection(conn: &Connection) -> ConnectionState {
    ConnectionState {
        conn_id: conn.source_id().to_vec(),
        state: match () {
            _ if conn.is_established() => ConnectionStatus::Established,
            _ if conn.is_in_early_data() => ConnectionStatus::Handshake,
            _ => ConnectionStatus::Handshake,
        },
        tls_session: conn.session().map(|s| s.to_vec()),
        streams: conn
            .streams()
            .map(|id| serialize_stream(conn, id))
            .collect(),
        flow_control: FlowControlState {
            max_data: conn.peer_max_data(),
            consumed: conn.local_max_data(),
        },
        recovery: RecoveryState {
            cwnd: conn.recovery_cwnd(),
            rtt_ms: conn.recovery_rtt().as_millis() as u64,
            ssthresh: conn.recovery_ssthresh(),
        },
        peer_addr: conn.peer_addr(),
        last_activity_ms: current_timestamp_ms(),
    }
}

pub fn deserialize_connection(state: ConnectionState) -> Connection {
    // Reconstruct connection from state
    // Note: This requires quiche to support state reconstruction
    // May need custom fork or C API wrapper

    let mut config = Config::new(PROTOCOL_VERSION).unwrap();

    // Restore TLS session for 0-RTT
    if let Some(session) = state.tls_session {
        config.set_session(&session);
    }

    // Create connection
    let mut conn = Connection::accept(
        &ConnectionId::from_vec(state.conn_id),
        None,
        state.peer_addr,
        state.peer_addr,  // Will be updated on first packet
        &mut config,
    ).unwrap();

    // Restore flow control
    conn.set_max_data(state.flow_control.max_data);

    // Restore stream states
    for stream in state.streams {
        restore_stream_state(&mut conn, stream);
    }

    conn
}
```

### 4.3 DynamoDB Storage

```rust
use aws_sdk_dynamodb::{Client as DynamoClient, types::AttributeValue};

pub struct ConnectionStateStore {
    client: DynamoClient,
    table_name: String,
}

impl ConnectionStateStore {
    pub fn new(client: DynamoClient, table_name: String) -> Self {
        Self { client, table_name }
    }

    pub async fn save(&self, conn_id: &str, state: &ConnectionState) -> Result<(), Error> {
        let item: HashMap<String, AttributeValue> = hashmap! {
            "conn_id".to_string() => AttributeValue::S(conn_id.to_string()),
            "state".to_string() => AttributeValue::S(serde_json::to_string(state)?),
            "ttl".to_string() => AttributeValue::N((current_timestamp_ms() + 3600000).to_string()),
        };

        self.client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(item))
            .send()
            .await?;

        Ok(())
    }

    pub async fn load(&self, conn_id: &str) -> Result<Option<ConnectionState>, Error> {
        let result = self.client
            .get_item()
            .table_name(&self.table_name)
            .key("conn_id", AttributeValue::S(conn_id.to_string()))
            .send()
            .await?;

        if let Some(item) = result.item {
            if let Some(AttributeValue::S(state_json)) = item.get("state") {
                return Ok(Some(serde_json::from_str(state_json)?));
            }
        }

        Ok(None)
    }

    pub async fn delete(&self, conn_id: &str) -> Result<(), Error> {
        self.client
            .delete_item()
            .table_name(&self.table_name)
            .key("conn_id", AttributeValue::S(conn_id.to_string()))
            .send()
            .await?;

        Ok(())
    }
}
```

---

## 5. Lambda Deployment

### 5.1 SAM Template

```yaml
AWSTemplateFormatVersion: '2010-09-09'
Transform: AWS::Serverless-2016-10-31

Resources:
  QuicProxyFunction:
    Type: AWS::Serverless::Function
    Properties:
      CodeUri: .
      Handler: bootstrap
      Runtime: provided.al2
      MemorySize: 256
      Timeout: 30
      Environment:
        Variables:
          RUST_LOG: info
          STATE_TABLE: !Ref ConnectionStateTable
      Events:
        UdpEvent:
          Type: EventBridgeRule
          Properties:
            Pattern:
              source:
                - aws.lambda-udp
            RetryPolicy:
              MaximumRetryAttempts: 0
      Policies:
        - DynamoDBCrudPolicy:
            TableName: !Ref ConnectionStateTable

  ConnectionStateTable:
    Type: AWS::DynamoDB::Table
    Properties:
      TableName: quic-connection-state
      BillingMode: PAY_PER_REQUEST
      AttributeDefinitions:
        - AttributeName: conn_id
          AttributeType: S
      KeySchema:
        - AttributeName: conn_id
          KeyType: HASH
      TimeToLiveSpecification:
        AttributeName: ttl
        Enabled: true

  UdpEndpoint:
    Type: AWS::Lambda::Url
    Properties:
      TargetFunctionArn: !GetAtt QuicProxyFunction.Arn
      AuthType: NONE
```

### 5.2 Build Script

```bash
#!/bin/bash
# build.sh

# Cross-compile for Lambda (x86_64)
cargo build --release --target x86_64-unknown-linux-musl

# Create bootstrap package
mkdir -p bootstrap/
cp target/x86_64-unknown-linux-musl/release/quic-proxy bootstrap/

# Package for Lambda
cd bootstrap/
zip -r ../lambda-package.zip .
cd ..

# Deploy
aws lambda update-function-code \
    --function-name QuicProxyFunction \
    --zip-file fileb://lambda-package.zip
```

### 5.3 Cargo Configuration

```toml
# Cargo.toml
[package]
name = "quic-proxy-lambda"
version = "0.1.0"
edition = "2021"

[dependencies]
quiche = { version = "0.26", features = ["ffi"] }
valtron = { path = "../../../ewe_platform/backends/foundation_core/src/valtron" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
aws-sdk-dynamodb = "1.0"
tokio = { version = "1", features = ["rt"] }  # Only for Lambda runtime

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true

# Cross-compilation for Lambda
[profile.release.package."*"]
[build]
target = "x86_64-unknown-linux-musl"
```

---

## 6. API Gateway Integration

### 6.1 HTTP/3 to HTTP/2 Proxy

```rust
pub struct QuicToHttpProxy {
    quic_conn: Option<Connection>,
    http_client: reqwest::Client,
    state_store: ConnectionStateStore,
}

impl QuicToHttpProxy {
    pub async fn handle_request(
        &mut self,
        request_headers: Vec<Header>,
        body: Option<Vec<u8>>,
    ) -> Result<ProxyResponse> {
        // Convert HTTP/3 headers to HTTP/2
        let mut http_req = self.http_client.request(
            Method::from_bytes(request_headers.get_method()?)?,
            request_headers.get_uri()?,
        );

        // Copy headers
        for header in &request_headers {
            if header.name().starts_with(b":") {
                continue;  // Skip pseudo-headers
            }
            http_req = http_req.header(
                header.name(),
                header.value(),
            );
        }

        // Add body if present
        if let Some(body) = body {
            http_req = http_req.body(body);
        }

        // Forward to origin
        let response = http_req.send().await?;

        // Convert response back to HTTP/3
        Ok(ProxyResponse {
            status: response.status().as_u16(),
            headers: response.headers()
                .iter()
                .map(|(k, v)| Header::new(k.as_ref(), v.as_bytes()))
                .collect(),
            body: response.bytes().await?.to_vec(),
        })
    }
}
```

### 6.2 WebSocket over QUIC

```rust
pub struct QuicWebSocketProxy {
    conn: Connection,
    ws_stream: Option<WebSocketStream>,
}

impl TaskIterator for QuicWebSocketProxy {
    type Ready = WsProxyResult;
    type Pending = ();

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending>> {
        // Handle QUIC stream for WebSocket frames
        // Convert WebSocket frames to QUIC STREAM frames
        // ...

        Some(TaskStatus::Pending(()))
    }
}
```

---

## 7. Cost Optimization

### 7.1 Connection Pooling

```rust
/// Keep warm connections for returning clients
pub struct ConnectionPool {
    connections: HashMap<ConnectionId, WarmConnection>,
    max_connections: usize,
}

pub struct WarmConnection {
    conn: Connection,
    last_used: Instant,
    access_count: usize,
}

impl ConnectionPool {
    pub fn get_or_create(&mut self, conn_id: &ConnectionId) -> &mut Connection {
        if let Some(warm) = self.connections.get_mut(conn_id) {
            warm.last_used = Instant::now();
            warm.access_count += 1;
            &mut warm.conn
        } else {
            // Evict LRU if at capacity
            if self.connections.len() >= self.max_connections {
                self.evict_lru();
            }

            // Create new warm connection
            self.connections.entry(conn_id.clone())
                .or_insert_with(|| WarmConnection {
                    conn: create_new_connection(),
                    last_used: Instant::now(),
                    access_count: 1,
                })
                .conn
        }
    }

    fn evict_lru(&mut self) {
        let oldest = self.connections
            .iter()
            .min_by_key(|(_, c)| c.last_used)
            .map(|(id, _)| id.clone());

        if let Some(id) = oldest {
            self.connections.remove(&id);
        }
    }
}
```

### 7.2 Provisioned Concurrency

```yaml
# SAM template with provisioned concurrency
QuicProxyFunction:
  Type: AWS::Serverless::Function
  Properties:
    ProvisionedConcurrencyConfig:
      ProvisionedConcurrentExecutions: 10
    ReservedConcurrentExecutions: 100
```

### 7.3 Cold Start Mitigation

```rust
// Pre-initialize expensive resources at startup
static mut CONFIG: Option<Config> = None;
static mut CRYPTO_CTX: Option<CryptoContext> = None;

fn init_once() {
    unsafe {
        CONFIG = Some(Config::new(PROTOCOL_VERSION).unwrap());
        CRYPTO_CTX = Some(CryptoContext::new().unwrap());
    }
}

#[no_mangle]
pub extern "C" fn lambda_init() {
    init_once();
}
```

---

## Summary

### Key Takeaways

1. **TaskIterator pattern** - State machine for QUIC event processing without async
2. **Lambda UDP events** - Custom runtime for UDP datagram delivery
3. **State serialization** - DynamoDB for connection state persistence
4. **No tokio** - Only Lambda runtime uses tokio, QUIC processing is sync
5. **Cost optimization** - Connection pooling, provisioned concurrency

### Deployment Checklist

- [ ] Build for x86_64-unknown-linux-musl
- [ ] Configure Lambda UDP event source
- [ ] Set up DynamoDB state table with TTL
- [ ] Configure provisioned concurrency for low latency
- [ ] Set up CloudWatch monitoring
- [ ] Test connection migration scenarios

---

## Further Reading

- [Valtron README](/home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron/README.md)
- [AWS Lambda Custom Runtime](https://docs.aws.amazon.com/lambda/latest/dg/runtimes-custom.html)
- [quiche FFI API](https://docs.quic.tech/quiche/)
- [DynamoDB TTL](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/TTL.html)
