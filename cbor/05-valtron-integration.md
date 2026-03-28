---
title: "CBOR Valtron Integration Guide"
subtitle: "Serialization tasks with Valtron executors - No async/await, no tokio"
based_on: "Valtron executor pattern from /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron/"
level: "Advanced - ewe_platform integration"
---

# CBOR Valtron Integration Guide

## Table of Contents

1. [Valtron Executor Overview](#1-valtron-executor-overview)
2. [CBOR Serialization Tasks](#2-cbor-serialization-tasks)
3. [Streaming with TaskIterator](#3-streaming-with-taskiterator)
4. [Integration Patterns](#4-integration-patterns)
5. [Complete Example: Message Protocol](#5-complete-example-message-protocol)
6. [Error Handling and Recovery](#6-error-handling-and-recovery)
7. [Performance Considerations](#7-performance-considerations)

---

## 1. Valtron Executor Overview

### 1.1 What is Valtron?

**Valtron** is an experimental async runtime built on iterators rather than async/await futures. It provides:

- `TaskIterator` trait for asynchronous tasks
- Single-threaded and multi-threaded executors
- No dependency on tokio or async/await
- Perfect for WASM, embedded, and Lambda environments

### 1.2 TaskIterator Pattern

```rust
use foundation_core::valtron::{TaskIterator, TaskStatus, NoSpawner};

struct Counter(usize, usize);

impl TaskIterator for Counter {
    type Pending = ();       // How task signals "not ready"
    type Ready = usize;      // The actual value type
    type Spawner = NoSpawner; // Sub-task spawning capability

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        let current = self.0;
        let next = current + 1;

        if next > self.1 {
            return None; // Task complete
        }

        self.0 = next;
        Some(TaskStatus::Ready(next))
    }
}
```

### 1.3 Executor Types

```rust
// Single-threaded executor (WASM, embedded)
use foundation_core::valtron::single::{initialize, spawn, run_until_complete};

initialize(seed);
spawn().with_task(task).schedule().unwrap();
run_until_complete();

// Multi-threaded executor
use foundation_core::valtron::multi::{block_on, get_pool};

block_on(seed, None, |pool| {
    pool.spawn().with_task(task).schedule().unwrap();
});
```

### 1.4 Why Valtron for CBOR?

```
Traditional async (tokio):
- Heavy runtime overhead
- Not Lambda-friendly
- Complex for simple serialization

Valtron approach:
- Lightweight iterator-based
- Lambda-compatible
- Simple serialization tasks
- No async/await needed
```

---

## 2. CBOR Serialization Tasks

### 2.1 Basic Encode Task

```rust
use foundation_core::valtron::{TaskIterator, TaskStatus, NoSpawner, FnReady};
use serde::Serialize;

/// Task to encode a value to CBOR bytes
pub struct CborEncodeTask<T> {
    data: Option<T>,
    result: Option<Result<Vec<u8>, serde_cbor::Error>>,
}

impl<T: Serialize> CborEncodeTask<T> {
    pub fn new(data: T) -> Self {
        Self {
            data: Some(data),
            result: None,
        }
    }
}

impl<T: Serialize + 'static> TaskIterator for CborEncodeTask<T> {
    type Ready = Result<Vec<u8>, serde_cbor::Error>;
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        if self.result.is_none() {
            // First call: perform encoding
            if let Some(data) = self.data.take() {
                self.result = Some(serde_cbor::to_vec(&data));
            }
        }

        // Return result if available
        self.result.take().map(TaskStatus::Ready)
    }
}

// Usage
use foundation_core::valtron::single::{initialize, spawn, run_until_complete};

fn encode_message() {
    let message = Message {
        id: 1,
        text: "Hello".to_string(),
    };

    initialize(42);

    spawn()
        .with_task(CborEncodeTask::new(message))
        .with_resolver(Box::new(FnReady::new(|result, _exec| {
            match result {
                Ok(bytes) => println!("Encoded: {} bytes", bytes.len()),
                Err(e) => eprintln!("Encode error: {}", e),
            }
        })))
        .schedule()
        .unwrap();

    run_until_complete();
}
```

### 2.2 Basic Decode Task

```rust
use serde::de::DeserializeOwned;

/// Task to decode CBOR bytes to a value
pub struct CborDecodeTask<T> {
    bytes: Option<Vec<u8>>,
    result: Option<Result<T, serde_cbor::Error>>,
    _marker: std::marker::PhantomData<T>,
}

impl<T: DeserializeOwned> CborDecodeTask<T> {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self {
            bytes: Some(bytes),
            result: None,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T: DeserializeOwned + 'static> TaskIterator for CborDecodeTask<T> {
    type Ready = Result<T, serde_cbor::Error>;
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        if self.result.is_none() {
            if let Some(bytes) = self.bytes.take() {
                self.result = Some(serde_cbor::from_slice(&bytes));
            }
        }

        self.result.take().map(TaskStatus::Ready)
    }
}

// Usage
fn decode_message(bytes: Vec<u8>) {
    initialize(42);

    spawn()
        .with_task(CborDecodeTask::<Message>::new(bytes))
        .with_resolver(Box::new(FnReady::new(|result, _exec| {
            match result {
                Ok(msg) => println!("Decoded: {:?}", msg),
                Err(e) => eprintln!("Decode error: {}", e),
            }
        })))
        .schedule()
        .unwrap();

    run_until_complete();
}
```

### 2.3 Round-Trip Task

```rust
/// Task that encodes and decodes for verification
pub struct CborRoundTripTask<T> {
    data: Option<T>,
    phase: RoundTripPhase,
}

enum RoundTripPhase {
    Encode,
    Decode(Vec<u8>),
    Done,
}

impl<T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug + 'static> TaskIterator
    for CborRoundTripTask<T>
{
    type Ready = Result<bool, String>; // true if round-trip successful
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        match std::mem::replace(&mut self.phase, RoundTripPhase::Done) {
            RoundTripPhase::Encode => {
                if let Some(data) = self.data.take() {
                    match serde_cbor::to_vec(&data) {
                        Ok(bytes) => {
                            self.phase = RoundTripPhase::Decode(bytes);
                            // Return Pending to continue
                            return Some(TaskStatus::Pending(()));
                        }
                        Err(e) => {
                            self.phase = RoundTripPhase::Done;
                            return Some(TaskStatus::Ready(Err(format!("Encode: {}", e))));
                        }
                    }
                }
            }
            RoundTripPhase::Decode(bytes) => {
                let original = self.data.take().unwrap(); // We cloned before
                match serde_cbor::from_slice::<T>(&bytes) {
                    Ok(decoded) => {
                        self.phase = RoundTripPhase::Done;
                        return Some(TaskStatus::Ready(Ok(decoded == original)));
                    }
                    Err(e) => {
                        self.phase = RoundTripPhase::Done;
                        return Some(TaskStatus::Ready(Err(format!("Decode: {}", e))));
                    }
                }
            }
            RoundTripPhase::Done => {}
        }

        None
    }
}
```

---

## 3. Streaming with TaskIterator

### 3.1 Stream Encoder Task

```rust
use std::io::Write;

/// Streaming encoder that processes multiple values
pub struct CborStreamEncoderTask<T, W> {
    items: std::vec::IntoIter<T>,
    writer: Option<W>,
    encoded_count: usize,
    error: Option<std::io::Error>,
}

impl<T: Serialize, W: Write> CborStreamEncoderTask<T, W> {
    pub fn new(items: Vec<T>, writer: W) -> Self {
        Self {
            items: items.into_iter(),
            writer: Some(writer),
            encoded_count: 0,
            error: None,
        }
    }
}

impl<T: Serialize + 'static, W: Write + 'static> TaskIterator
    for CborStreamEncoderTask<T, W>
{
    type Ready = usize; // Number of items encoded
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        if let Some(ref mut writer) = self.writer {
            // Encode next item
            if let Some(item) = self.items.next() {
                match serde_cbor::to_writer(writer, &item) {
                    Ok(()) => {
                        self.encoded_count += 1;
                        Some(TaskStatus::Ready(self.encoded_count))
                    }
                    Err(e) => {
                        self.error = Some(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            e.to_string(),
                        ));
                        Some(TaskStatus::Ready(self.encoded_count))
                    }
                }
            } else {
                // No more items
                self.writer = None;
                None
            }
        } else {
            None
        }
    }
}
```

### 3.2 Stream Decoder Task

```rust
use std::io::Read;

/// Streaming decoder that yields values one at a time
pub struct CborStreamDecoderTask<R: Read> {
    reader: R,
    buffer: Vec<u8>,
    offset: usize,
}

impl<R: Read> CborStreamDecoderTask<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            buffer: Vec::with_capacity(4096),
            offset: 0,
        }
    }

    fn read_next_value(&mut self) -> Option<Vec<u8>> {
        // Simplified: In reality, need to parse CBOR length prefix
        let mut buf = [0u8; 1024];
        match self.reader.read(&mut buf) {
            Ok(0) => None,
            Ok(n) => {
                self.buffer.extend_from_slice(&buf[..n]);
                Some(self.buffer.split_off(0))
            }
            Err(_) => None,
        }
    }
}

impl<R: Read + 'static> TaskIterator for CborStreamDecoderTask<R> {
    type Ready = Option<Vec<u8>>; // Next CBOR value bytes
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        match self.read_next_value() {
            Some(bytes) => Some(TaskStatus::Ready(Some(bytes))),
            None => Some(TaskStatus::Ready(None)),
        }
    }
}
```

### 3.3 Newline-Delimited CBOR (CBORL) Task

```rust
use std::io::{BufRead, BufReader, Read, Write};

/// ND-CBOR decoder task
pub struct NdcbrDecodeTask<R: Read, T> {
    reader: Option<BufReader<R>>,
    line_count: usize,
    _marker: std::marker::PhantomData<T>,
}

impl<R: Read, T: DeserializeOwned> NdcbrDecodeTask<R, T> {
    pub fn new(reader: R) -> Self {
        Self {
            reader: Some(BufReader::new(reader)),
            line_count: 0,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<R: Read + 'static, T: DeserializeOwned + 'static> TaskIterator
    for NdcbrDecodeTask<R, T>
{
    type Ready = Option<Result<T, serde_cbor::Error>>;
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        if let Some(ref mut reader) = self.reader {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    // EOF
                    self.reader = None;
                    Some(TaskStatus::Ready(None))
                }
                Ok(_) => {
                    self.line_count += 1;
                    let bytes = line.trim().as_bytes().to_vec();
                    let result = serde_cbor::from_slice(&bytes);
                    Some(TaskStatus::Ready(Some(result)))
                }
                Err(_) => {
                    self.reader = None;
                    Some(TaskStatus::Ready(Some(Err(serde_cbor::Error::message(
                        "IO error",
                    )))))
                }
            }
        } else {
            None
        }
    }
}
```

---

## 4. Integration Patterns

### 4.1 Request/Response Pattern

```rust
use foundation_core::valtron::single::{spawn, run_until_complete};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
struct Request {
    id: u64,
    method: String,
    params: serde_cbor::Value,
}

#[derive(Serialize, Deserialize, Debug)]
struct Response {
    id: u64,
    result: serde_cbor::Value,
}

/// Request/Response handler task
pub struct RequestHandlerTask {
    request_bytes: Option<Vec<u8>>,
}

impl TaskIterator for RequestHandlerTask {
    type Ready = Vec<u8>; // Response bytes
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        if let Some(bytes) = self.request_bytes.take() {
            // Decode request
            let request: Request = match serde_cbor::from_slice(&bytes) {
                Ok(r) => r,
                Err(e) => {
                    let error_response = Response {
                        id: 0,
                        result: serde_cbor::Value::Text(format!("Error: {}", e)),
                    };
                    return Some(TaskStatus::Ready(
                        serde_cbor::to_vec(&error_response).unwrap(),
                    ));
                }
            };

            // Process request (simplified)
            let result = serde_cbor::Value::Text(format!(
                "Processed {} with {} params",
                request.method,
                match &request.params {
                    serde_cbor::Value::Array(a) => a.len(),
                    _ => 0,
                }
            ));

            // Encode response
            let response = Response {
                id: request.id,
                result,
            };

            Some(TaskStatus::Ready(serde_cbor::to_vec(&response).unwrap()))
        } else {
            None
        }
    }
}

// Usage
fn handle_request(request_bytes: Vec<u8>) -> Vec<u8> {
    let mut response_bytes: Option<Vec<u8>> = None;

    spawn()
        .with_task(RequestHandlerTask {
            request_bytes: Some(request_bytes),
        })
        .with_resolver(Box::new(FnReady::new(|bytes, _exec| {
            response_bytes = Some(bytes);
        })))
        .schedule()
        .unwrap();

    run_until_complete();
    response_bytes.unwrap()
}
```

### 4.2 Batch Processing Pattern

```rust
/// Batch encode multiple items
pub struct BatchEncodeTask<T> {
    items: Vec<T>,
    batch_size: usize,
    current_batch: usize,
}

impl<T: Serialize> BatchEncodeTask<T> {
    pub fn new(items: Vec<T>, batch_size: usize) -> Self {
        Self {
            items,
            batch_size,
            current_batch: 0,
        }
    }
}

impl<T: Serialize + 'static> TaskIterator for BatchEncodeTask<T> {
    type Ready = Vec<u8>; // Batch encoded bytes
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        let start = self.current_batch * self.batch_size;
        if start >= self.items.len() {
            return None;
        }

        let end = (start + self.batch_size).min(self.items.len());
        let batch: Vec<&T> = self.items[start..end].iter().collect();

        self.current_batch += 1;

        // Encode batch as CBOR array
        Some(TaskStatus::Ready(serde_cbor::to_vec(&batch).unwrap()))
    }
}

// Usage
fn batch_process() {
    let items: Vec<Message> = (0..1000)
        .map(|i| Message { id: i, text: format!("Message {}", i) })
        .collect();

    spawn()
        .with_task(BatchEncodeTask::new(items, 100)) // 100 items per batch
        .with_resolver(Box::new(FnReady::new(|batch_bytes, _exec| {
            println!("Batch encoded: {} bytes", batch_bytes.len());
        })))
        .schedule()
        .unwrap();

    run_until_complete();
}
```

### 4.3 Pipeline Pattern

```rust
/// CBOR processing pipeline
pub struct CborPipeline<Input, Output> {
    decode_task: Option<CborDecodeTask<Input>>,
    process_fn: Option<Box<dyn Fn(Input) -> Output>>,
    encode_task: Option<CborEncodeTask<Output>>,
    state: PipelineState,
}

enum PipelineState {
    Decode,
    Process,
    Encode,
    Done,
}

impl<Input, Output> TaskIterator for CborPipeline<Input, Output>
where
    Input: DeserializeOwned + 'static,
    Output: Serialize + 'static,
{
    type Ready = Vec<u8>;
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        match std::mem::replace(&mut self.state, PipelineState::Done) {
            PipelineState::Decode => {
                // Decode handled externally, transition to process
                self.state = PipelineState::Process;
                Some(TaskStatus::Pending(()))
            }
            PipelineState::Process => {
                // Process handled externally, transition to encode
                self.state = PipelineState::Encode;
                Some(TaskStatus::Pending(()))
            }
            PipelineState::Encode => {
                // Encode handled externally
                self.state = PipelineState::Done;
                None
            }
            PipelineState::Done => None,
        }
    }
}
```

---

## 5. Complete Example: Message Protocol

### 5.1 Protocol Definition

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MessageType {
    Request,
    Response,
    Error,
    Heartbeat,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub msg_type: MessageType,
    pub id: u64,
    pub timestamp: u64,
    pub payload: Vec<u8>,
    pub signature: Option<Vec<u8>>,
}

impl Message {
    pub fn request(id: u64, payload: Vec<u8>) -> Self {
        Self {
            msg_type: MessageType::Request,
            id,
            timestamp: current_timestamp(),
            payload,
            signature: None,
        }
    }

    pub fn response(id: u64, payload: Vec<u8>) -> Self {
        Self {
            msg_type: MessageType::Response,
            id,
            timestamp: current_timestamp(),
            payload,
            signature: None,
        }
    }

    pub fn error(id: u64, message: &str) -> Self {
        Self {
            msg_type: MessageType::Error,
            id,
            timestamp: current_timestamp(),
            payload: message.as_bytes().to_vec(),
            signature: None,
        }
    }
}

fn current_timestamp() -> u64 {
    // In production, use actual time
    1632844800
}
```

### 5.2 Protocol Handler

```rust
use foundation_core::valtron::{TaskIterator, TaskStatus, NoSpawner, FnReady};
use foundation_core::valtron::single::{spawn, run_until_complete};

/// Message protocol handler
pub struct ProtocolHandler {
    input: Option<Vec<u8>>,
    state: HandlerState,
}

enum HandlerState {
    Decode,
    Validate,
    Process,
    Encode,
    Done,
}

impl TaskIterator for ProtocolHandler {
    type Ready = Vec<u8>;
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        match self.state {
            HandlerState::Decode => {
                if let Some(bytes) = self.input.take() {
                    // Decode message
                    match serde_cbor::from_slice::<Message>(&bytes) {
                        Ok(msg) => {
                            println!("Decoded message: {:?}", msg);
                            self.state = HandlerState::Validate;
                            Some(TaskStatus::Pending(()))
                        }
                        Err(e) => {
                            self.state = HandlerState::Done;
                            // Return error message
                            let error = Message::error(0, &format!("Decode error: {}", e));
                            Some(TaskStatus::Ready(serde_cbor::to_vec(&error).unwrap()))
                        }
                    }
                } else {
                    self.state = HandlerState::Done;
                    None
                }
            }
            HandlerState::Validate => {
                // Validate message (signature, etc.)
                println!("Validating message...");
                self.state = HandlerState::Process;
                Some(TaskStatus::Pending(()))
            }
            HandlerState::Process => {
                // Process message and create response
                println!("Processing message...");
                let response = Message::response(1, b"OK".to_vec());
                self.state = HandlerState::Encode;

                // Store response for encoding
                // In real implementation, would use shared state
                Some(TaskStatus::Ready(serde_cbor::to_vec(&response).unwrap()))
            }
            HandlerState::Encode => {
                self.state = HandlerState::Done;
                None
            }
            HandlerState::Done => None,
        }
    }
}

// Usage
fn run_protocol() {
    let request = Message::request(1, vec![1, 2, 3, 4]);
    let request_bytes = serde_cbor::to_vec(&request).unwrap();

    spawn()
        .with_task(ProtocolHandler {
            input: Some(request_bytes),
            state: HandlerState::Decode,
        })
        .with_resolver(Box::new(FnReady::new(|response_bytes, _exec| {
            let response: Message = serde_cbor::from_slice(&response_bytes).unwrap();
            println!("Response: {:?}", response);
        })))
        .schedule()
        .unwrap();

    run_until_complete();
}
```

### 5.3 COSE Integration

```rust
/// Secure message handler with COSE verification
pub struct SecureMessageHandler {
    input: Option<Vec<u8>>,
    public_key: Vec<u8>,
}

impl TaskIterator for SecureMessageHandler {
    type Ready = Result<Vec<u8>, String>;
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        if let Some(bytes) = self.input.take() {
            // Verify COSE signature first
            // Note: This is pseudocode - use actual COSE library
            let verified_payload = match verify_cose_signature(&bytes, &self.public_key) {
                Ok(payload) => payload,
                Err(e) => return Some(TaskStatus::Ready(Err(format!("Signature verify: {}", e)))),
            };

            // Decode message from verified payload
            match serde_cbor::from_slice::<Message>(&verified_payload) {
                Ok(msg) => Some(TaskStatus::Ready(Ok(serde_cbor::to_vec(&msg).unwrap()))),
                Err(e) => Some(TaskStatus::Ready(Err(format!("Decode: {}", e)))),
            }
        } else {
            None
        }
    }
}

fn verify_cose_signature(cose_bytes: &[u8], public_key: &[u8]) -> Result<Vec<u8>, String> {
    // In production, use actual COSE library
    // This is a placeholder
    Ok(cose_bytes.to_vec())
}
```

---

## 6. Error Handling and Recovery

### 6.1 Error Types

```rust
use foundation_core::valtron::{TaskIterator, TaskStatus, NoSpawner};

#[derive(Debug)]
pub enum CborTaskError {
    Encode(serde_cbor::Error),
    Decode(serde_cbor::Error),
    Validation(String),
    Io(std::io::Error),
}

/// Task with proper error handling
pub struct RobustDecodeTask<T> {
    bytes: Option<Vec<u8>>,
    result: Option<Result<T, CborTaskError>>,
    _marker: std::marker::PhantomData<T>,
}

impl<T: DeserializeOwned> RobustDecodeTask<T> {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self {
            bytes: Some(bytes),
            result: None,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T: DeserializeOwned + 'static> TaskIterator for RobustDecodeTask<T> {
    type Ready = Result<T, CborTaskError>;
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        if self.result.is_none() {
            if let Some(bytes) = self.bytes.take() {
                // Validate input size first
                if bytes.len() > 10 * 1024 * 1024 {
                    self.result = Some(Err(CborTaskError::Validation(
                        "Input too large (>10MB)".to_string(),
                    )));
                } else {
                    self.result = Some(
                        serde_cbor::from_slice(&bytes)
                            .map_err(CborTaskError::Decode),
                    );
                }
            }
        }

        self.result.take().map(TaskStatus::Ready)
    }
}
```

### 6.2 Retry Logic

```rust
/// Task with retry on failure
pub struct RetryDecodeTask<T> {
    bytes: Vec<u8>,
    attempts: u32,
    max_attempts: u32,
    result: Option<Result<T, CborTaskError>>,
    _marker: std::marker::PhantomData<T>,
}

impl<T: DeserializeOwned> RetryDecodeTask<T> {
    pub fn new(bytes: Vec<u8>, max_attempts: u32) -> Self {
        Self {
            bytes,
            attempts: 0,
            max_attempts,
            result: None,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T: DeserializeOwned + 'static> TaskIterator for RetryDecodeTask<T> {
    type Ready = Result<T, CborTaskError>;
    type Pending = u32; // Retry delay
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        if self.attempts >= self.max_attempts {
            return self.result.take().map(TaskStatus::Ready);
        }

        self.attempts += 1;

        match serde_cbor::from_slice(&self.bytes) {
            Ok(value) => {
                self.result = Some(Ok(value));
                Some(TaskStatus::Ready(Ok(value)))
            }
            Err(e) => {
                if self.attempts < self.max_attempts {
                    // Return Pending with retry delay
                    Some(TaskStatus::Pending(self.attempts))
                } else {
                    self.result = Some(Err(CborTaskError::Decode(e)));
                    self.result.clone().map(TaskStatus::Ready)
                }
            }
        }
    }
}
```

---

## 7. Performance Considerations

### 7.1 Task Pooling

```rust
/// Pool of reusable encode tasks
pub struct EncodeTaskPool<T> {
    pool: Vec<Option<CborEncodeTask<T>>>,
}

impl<T: Serialize> EncodeTaskPool<T> {
    pub fn new(size: usize) -> Self {
        Self {
            pool: (0..size).map(|_| None).collect(),
        }
    }

    pub fn acquire(&mut self, data: T) -> Option<CborEncodeTask<T>> {
        self.pool.iter_mut().find_map(|slot| slot.take()).map(|mut task| {
            task.data = Some(data);
            task.result = None;
            task
        })
    }

    pub fn release(&mut self, mut task: CborEncodeTask<T>) {
        task.data = None;
        task.result = None;
        for slot in &mut self.pool {
            if slot.is_none() {
                *slot = Some(task);
                return;
            }
        }
    }
}
```

### 7.2 Memory-Efficient Streaming

```rust
/// Memory-efficient stream processor
pub struct StreamProcessor<R: Read, W: Write> {
    reader: R,
    writer: W,
    buffer: Vec<u8>,
    processed: usize,
}

impl<R: Read + 'static, W: Write + 'static> TaskIterator for StreamProcessor<R, W> {
    type Ready = usize; // Items processed
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        // Read one CBOR value at a time
        let mut buf = [0u8; 1024];
        match self.reader.read(&mut buf) {
            Ok(0) => None, // EOF
            Ok(n) => {
                // Process and write
                self.buffer.extend_from_slice(&buf[..n]);
                self.writer.write_all(&buf[..n]).unwrap();
                self.processed += 1;
                Some(TaskStatus::Ready(self.processed))
            }
            Err(_) => None,
        }
    }
}
```

---

## Appendix A: Valtron CBOR Quick Reference

```rust
// Basic encode task
spawn().with_task(CborEncodeTask::new(data)).schedule().unwrap();

// Basic decode task
spawn().with_task(CborDecodeTask::<T>::new(bytes)).schedule().unwrap();

// Streaming encode
spawn().with_task(CborStreamEncoderTask::new(items, writer)).schedule().unwrap();

// With resolver
spawn()
    .with_task(task)
    .with_resolver(Box::new(FnReady::new(|result, exec| {
        // Handle result
    })))
    .schedule()
    .unwrap();

// Run to completion
run_until_complete();
```

---

*This completes the CBOR exploration. Refer to [exploration.md](exploration.md) for the complete index.*
