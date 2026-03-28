---
title: "Rust Revision: Lambda Runtime Without Tokio"
subtitle: "Replicating Lambda Rust Runtime functionality using Valtron TaskIterator instead of async/await"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/aws/aws-lambda-rust-runtime/rust-revision.md
related:
  - /home/darkvoid/Boxxed/@dev/repo-expolorations/aws/aws-lambda-rust-runtime/exploration.md
  - /home/darkvoid/Boxxed/@dev/repo-expolorations/alchemy/fragment/07-valtron-executor-guide.md
  - /home/darkvoid/Boxxed/@dev/ewe_platform/specifications/08-valtron-async-iterators/requirements.md
---

# Rust Revision: Lambda Runtime Without Tokio

## Executive Summary

This document demonstrates how to replicate Lambda Rust Runtime functionality **without Tokio or async/await** using Valtron's TaskIterator pattern. The key insight is that Lambda's request/response flow is a simple **state machine**, not a complex async computation.

### Why Valtron for Lambda Runtimes?

| Aspect | Tokio Runtime | Valtron TaskIterator |
|--------|--------------|---------------------|
| **Runtime Overhead** | ~1-10ms initialization | Zero (pure struct) |
| **Cold Start** | Runtime + handler init | Handler init only |
| **Binary Size** | Tokio adds ~500KB | No runtime dependency |
| **WASM Compatible** | No | Yes |
| **Mental Model** | async/await, Futures | Explicit state machine |
| **Lambda Fit** | Overkill | Perfect match |

---

## Part 1: Mental Model Shift

### From Async Handler to TaskIterator

```rust
// Tokio async handler (traditional)
#[tokio::main]
async fn main() -> Result<(), Error> {
    let func = service_fn(my_handler);
    lambda_runtime::run(func).await?;
    Ok(())
}

async fn my_handler(event: LambdaEvent<Value>) -> Result<Value, Error> {
    let data = fetch_data().await?;  // async call
    let processed = process(data).await?;  // async call
    Ok(processed)
}

// Valtron TaskIterator (no async)
fn main() -> Result<(), Error> {
    let task = HandlerTask::new();
    valtron::single::initialize_pool(42);
    valtron::single::spawn().with_task(task).schedule()?;
    valtron::single::run_until_complete();
    Ok(())
}

struct HandlerTask {
    state: HandlerState,
}

impl TaskIterator for HandlerTask {
    type Ready = Value;
    type Pending = HandlerState;
    type Spawner = NoSpawner;

    fn next_status(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        match &self.state {
            HandlerState::GetEvent => {
                // Poll Runtime API (sync HTTP)
                let event = sync_get_next_invocation();
                self.state = HandlerState::Processing { event };
                Some(TaskStatus::Pending(HandlerState::Processing { event: Value::Null }))
            }
            HandlerState::Processing { event } => {
                let result = process_sync(event);
                Some(TaskStatus::Ready(result))
            }
            HandlerState::Done => None,
        }
    }
}
```

### Understanding the Pattern

```
async/await hides the state machine:
  async fn handler() {
      let a = fetch_a().await;  // State 1: waiting for A
      let b = fetch_b().await;  // State 2: waiting for B
      return a + b;             // State 3: complete
  }

  // Compiler generates hidden state machine

TaskIterator makes it explicit:
  enum HandlerState {
      FetchingA,
      FetchingB { a: Value },
      Complete { result: Value },
  }

  fn next_status(&mut self) -> Option<TaskStatus<...>> {
      // Explicit state transitions
  }
```

---

## Part 2: Runtime Loop as TaskIterator

### Lambda Runtime State Machine

```rust
pub enum RuntimeState {
    /// Initial state
    Initializing,

    /// Polling Runtime API for next invocation
    PollingNextInvocation,

    /// Received event, deserializing
    DeserializingEvent { raw_body: Vec<u8> },

    /// Calling handler service
    CallingHandler { event: Value },

    /// Waiting for handler result
    AwaitingHandlerResult,

    /// Serializing response
    SerializingResponse { result: Value },

    /// Sending response to Runtime API
    SendingResponse { request_id: String, body: Vec<u8> },

    /// Error handling
    HandlingError { request_id: String, error: String },

    /// Ready to process next invocation
    ReadyForNext,

    /// Terminal state
    Done,
}
```

### Complete Runtime TaskIterator

```rust
use foundation_core::valtron::{TaskIterator, TaskStatus, NoSpawner};
use std::cell::RefCell;
use std::rc::Rc;

pub struct LambdaRuntimeTask {
    state: RuntimeState,
    config: RuntimeConfig,
    client: SyncHttpClient,
    invocations_processed: Rc<RefCell<u64>>,
}

impl LambdaRuntimeTask {
    pub fn new() -> Self {
        Self {
            state: RuntimeState::Initializing,
            config: RuntimeConfig::from_env(),
            client: SyncHttpClient::new("localhost", 9001),
            invocations_processed: Rc::new(RefCell::new(0)),
        }
    }
}

impl TaskIterator for LambdaRuntimeTask {
    type Ready = InvocationResult;
    type Pending = RuntimeState;
    type Spawner = NoSpawner;

    fn next_status(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        match std::mem::replace(&mut self.state, RuntimeState::Done) {
            RuntimeState::Initializing => {
                tracing::info!("Lambda Rust Runtime (Valtron) initializing");
                self.state = RuntimeState::PollingNextInvocation;
                Some(TaskStatus::Pending(RuntimeState::PollingNextInvocation))
            }

            RuntimeState::PollingNextInvocation => {
                match sync_get_next_invocation(&self.client) {
                    Ok((request_id, raw_body)) => {
                        self.state = RuntimeState::DeserializingEvent { raw_body };
                        Some(TaskStatus::Pending(RuntimeState::DeserializingEvent {
                            raw_body: vec![]
                        }))
                    }
                    Err(e) => {
                        tracing::error!("Failed to get invocation: {}", e);
                        self.state = RuntimeState::PollingNextInvocation;
                        Some(TaskStatus::Pending(RuntimeState::PollingNextInvocation))
                    }
                }
            }

            RuntimeState::DeserializingEvent { raw_body } => {
                let event: Value = serde_json::from_slice(&raw_body).unwrap_or(Value::Null);
                self.current_request_id = Some(self.extract_request_id());
                self.state = RuntimeState::CallingHandler { event };
                Some(TaskStatus::Pending(RuntimeState::CallingHandler { event: Value::Null }))
            }

            RuntimeState::CallingHandler { event } => {
                // Call user's handler function (synchronous)
                let result = self.call_handler(event);
                self.state = RuntimeState::SerializingResponse { result };
                Some(TaskStatus::Pending(RuntimeState::SerializingResponse {
                    result: Value::Null
                }))
            }

            RuntimeState::SerializingResponse { result } => {
                let body = serde_json::to_vec(&result).unwrap_or_default();
                let request_id = self.current_request_id.clone().unwrap_or_default();
                self.state = RuntimeState::SendingResponse { request_id, body };
                Some(TaskStatus::Pending(RuntimeState::SendingResponse {
                    request_id: String::new(),
                    body: vec![]
                }))
            }

            RuntimeState::SendingResponse { request_id, body } => {
                match sync_send_response(&self.client, &request_id, &body) {
                    Ok(()) => {
                        *self.invocations_processed.borrow_mut() += 1;
                        self.state = RuntimeState::ReadyForNext;
                        Some(TaskStatus::Ready(InvocationResult {
                            request_id,
                            status: "success".to_string(),
                        }))
                    }
                    Err(e) => {
                        tracing::error!("Failed to send response: {}", e);
                        self.state = RuntimeState::PollingNextInvocation;
                        Some(TaskStatus::Pending(RuntimeState::PollingNextInvocation))
                    }
                }
            }

            RuntimeState::ReadyForNext | RuntimeState::HandlingError { .. } => {
                self.state = RuntimeState::PollingNextInvocation;
                Some(TaskStatus::Pending(RuntimeState::PollingNextInvocation))
            }

            RuntimeState::Done => None,
            _ => None,
        }
    }
}
```

---

## Part 3: Synchronous HTTP Client

### Sync Runtime API Client

```rust
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

pub struct SyncHttpClient {
    host: String,
    port: u16,
    timeout: Duration,
}

impl SyncHttpClient {
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            host: host.to_string(),
            port,
            timeout: Duration::from_secs(30),
        }
    }

    /// GET next invocation from Runtime API
    pub fn get_next_invocation(&self) -> Result<(String, Vec<u8>), HttpError> {
        let request = format!(
            "GET /2018-06-01/runtime/invocation/next HTTP/1.1\r\n\
             Host: {}\r\n\
             Connection: close\r\n\r\n",
            self.host
        );

        let response = self.send_request(&request)?;
        let (headers, body) = self.parse_response(&response)?;

        let request_id = headers.get("lambda-runtime-aws-request-id")
            .ok_or(HttpError::MissingHeader)?
            .clone();

        Ok((request_id, body))
    }

    /// POST response to Runtime API
    pub fn send_response(&self, request_id: &str, body: &[u8]) -> Result<(), HttpError> {
        let request = format!(
            "POST /2018-06-01/runtime/invocation/{}/response HTTP/1.1\r\n\
             Host: {}\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\r\n",
            request_id, self.host, body.len()
        );

        let mut full_request = request.into_bytes();
        full_request.extend_from_slice(body);

        self.send_request(&String::from_utf8_lossy(&full_request))?;
        Ok(())
    }

    fn send_request(&self, request: &str) -> Result<Vec<u8>, HttpError> {
        let addr = format!("{}:{}", self.host, self.port);
        let mut stream = TcpStream::connect_timeout(&addr, self.timeout)?;
        stream.set_read_timeout(Some(self.timeout))?;
        stream.set_write_timeout(Some(self.timeout))?;

        stream.write_all(request.as_bytes())?;

        let mut response = Vec::new();
        stream.read_to_end(&mut response)?;

        Ok(response)
    }

    fn parse_response(&self, data: &[u8]) -> Result<(HashMap<String, String>, Vec<u8>), HttpError> {
        // Parse HTTP response headers and body
        // Simplified for brevity
        let mut headers = HashMap::new();
        let body_start = data.windows(4).position(|w| w == b"\r\n\r\n")
            .map(|p| p + 4)
            .unwrap_or(data.len());

        // Extract headers
        let header_section = std::str::from_utf8(&data[..body_start - 4])?;
        for line in header_section.lines().skip(1) {
            if let Some((key, value)) = line.split_once(": ") {
                headers.insert(key.to_lowercase(), value.to_string());
            }
        }

        Ok((headers, data[body_start..].to_vec()))
    }
}
```

---

## Part 4: Handler Function Abstraction

### Sync Handler Trait

```rust
/// Trait for synchronous Lambda handlers
pub trait SyncHandler<Event, Response, Error> {
    fn handle(&self, event: Event) -> Result<Response, Error>;
}

/// Function pointer implementation
impl<F, Event, Response, Err> SyncHandler<Event, Response, Err> for F
where
    F: Fn(Event) -> Result<Response, Err>,
{
    fn handle(&self, event: Event) -> Result<Response, Err> {
        self(event)
    }
}
```

### Service Function Helper

```rust
pub fn service_fn_sync<F, Event, Response, Err>(f: F) -> SyncServiceFn<F>
where
    F: Fn(Event) -> Result<Response, Err>,
{
    SyncServiceFn { f }
}

pub struct SyncServiceFn<F> {
    f: F,
}

impl<F, Event, Response, Err> SyncHandler<Event, Response, Err> for SyncServiceFn<F>
where
    F: Fn(Event) -> Result<Response, Err>,
{
    fn handle(&self, event: Event) -> Result<Response, Err> {
        (self.f)(event)
    }
}
```

---

## Part 5: Complete Example

### Hello World without Async

```rust
use foundation_core::valtron::{execute, TaskIterator, TaskStatus, NoSpawner};
use serde_json::{json, Value};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let task = LambdaRuntimeTask::with_handler(service_fn_sync(handler));

    let mut stream = execute(task, None)?;
    for item in stream {
        if let Stream::Next(result) = item {
            println!("Completed: {}", result.request_id);
        }
    }

    Ok(())
}

fn handler(event: LambdaEvent<Value>) -> Result<Value, Box<dyn std::error::Error>> {
    let (event, _context) = event.into_parts();
    let first_name = event["firstName"].as_str().unwrap_or("world");
    Ok(json!({ "message": format!("Hello, {}!", first_name) }))
}
```

---

## Summary

| Tokio Pattern | Valtron Equivalent |
|---------------|-------------------|
| `#[tokio::main]` | `valtron::single::initialize_pool()` |
| `async fn handler()` | `fn handler()` + `SyncHandler` trait |
| `lambda_runtime::run()` | `execute(task)` |
| `service_fn()` | `service_fn_sync()` |
| `await` | Explicit state transitions |
| Tower Service | Direct function call |

---

*See [04-valtron-integration.md](04-valtron-integration.md) for the complete Valtron integration guide.*
