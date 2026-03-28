---
title: "Valtron Executor Guide"
subtitle: "TaskIterator pattern for async-like execution without async/await"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/wildcard-ai/07-valtron-executor-guide.md
prerequisites: Understanding of valtron README, rust-revision.md
---

# Valtron Executor Guide

## Introduction

Valtron is an iterator-based async runtime that doesn't use async/await or futures. Instead, it uses the **TaskIterator** trait for task execution.

### Why Valtron?

| Aspect | Tokio/Async | Valtron |
|--------|-------------|---------|
| **Runtime** | Future/poll | Iterator/next |
| **Syntax** | async/await | Iterator trait |
| **Binary Size** | Large (~5-10MB) | Small (~100KB) |
| **Cold Start** | Slow | Fast |
| **Threading** | Multi-threaded | Single or Multi |
| **Complexity** | High | Low |

---

## TaskIterator Trait

### Definition

```rust
use foundation_core::valtron::{TaskStatus, NoSpawner};

pub enum TaskStatus<Ready, Pending, Spawner> {
    Ready(Ready),
    Pending(Pending),
    Spawn(Spawner),
}

pub trait TaskIterator {
    type Ready;
    type Pending;
    type Spawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>>;
}
```

### Types

| Type | Description |
|------|-------------|
| `Ready` | The value produced when task completes |
| `Pending` | State while task is working |
| `Spawner` | Handle to spawn sub-tasks (use `NoSpawner` if not needed) |

---

## Basic Example: Counter

```rust
use foundation_core::valtron::{TaskIterator, TaskStatus, NoSpawner};

struct Counter {
    current: usize,
    max: usize,
}

impl Counter {
    fn new(max: usize) -> Self {
        Self { current: 0, max }
    }
}

impl TaskIterator for Counter {
    type Ready = usize;
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        if self.current >= self.max {
            return None;  // Task complete
        }

        self.current += 1;
        Some(TaskStatus::Ready(self.current))
    }
}
```

### Usage

```rust
use foundation_core::valtron::single::{initialize, spawn, run_until_complete};
use foundation_core::valtron::FnReady;

fn main() {
    initialize(42);

    spawn()
        .with_task(Counter::new(5))
        .with_resolver(Box::new(FnReady::new(|value, _| {
            println!("Count: {}", value);
        })))
        .schedule()
        .unwrap();

    run_until_complete();
}
```

---

## HTTP Task Example

### Blocking HTTP Request

```rust
use foundation_core::valtron::{TaskIterator, TaskStatus, NoSpawner};
use reqwest::blocking::{Client, Response};
use serde_json::Value;

pub struct HttpTask {
    client: Client,
    method: String,
    url: String,
    headers: std::collections::HashMap<String, String>,
    body: Option<Value>,
    executed: bool,
}

impl HttpTask {
    pub fn new(
        method: String,
        url: String,
        headers: std::collections::HashMap<String, String>,
        body: Option<Value>,
    ) -> Self {
        Self {
            client: Client::new(),
            method,
            url,
            headers,
            body,
            executed: false,
        }
    }
}

impl TaskIterator for HttpTask {
    type Ready = Value;
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        if self.executed {
            return None;
        }

        self.executed = true;

        // Build request
        let mut request = match self.method.as_str() {
            "GET" => self.client.get(&self.url),
            "POST" => self.client.post(&self.url),
            "PUT" => self.client.put(&self.url),
            "DELETE" => self.client.delete(&self.url),
            _ => panic!("Unsupported method"),
        };

        // Add headers
        for (key, value) in &self.headers {
            request = request.header(key, value);
        }

        // Add body
        if let Some(body) = &self.body {
            request = request.json(body);
        }

        // Execute and return
        match request.send() {
            Ok(resp) => {
                let result: Value = resp.json().unwrap_or(json!({"error": "Parse failed"}));
                Some(TaskStatus::Ready(result))
            }
            Err(e) => {
                Some(TaskStatus::Ready(json!({"error": e.to_string()})))
            }
        }
    }
}
```

### Usage

```rust
fn main() {
    initialize(42);

    let mut headers = std::collections::HashMap::new();
    headers.insert("Authorization".to_string(), "Bearer token123".to_string());

    let task = HttpTask::new(
        "GET".to_string(),
        "https://api.example.com/data".to_string(),
        headers,
        None,
    );

    let mut result = None;
    for status in spawn().with_task(task).iter() {
        if let TaskStatus::Ready(value) = status {
            result = Some(value);
            break;
        }
    }

    println!("Result: {:?}", result);
}
```

---

## Flow Execution Task

### Complete Implementation

```rust
use foundation_core::valtron::{TaskIterator, TaskStatus, NoSpawner};
use serde_json::{json, Value};
use std::collections::HashMap;

pub struct FlowExecutionTask {
    flow: Flow,
    execution_trace: HashMap<String, ActionTrace>,
    current_action_index: usize,
    client: reqwest::blocking::Client,
    auth: AuthConfig,
    parameters: Value,
    request_body: Value,
}

pub struct ActionTrace {
    pub parameters: Value,
    pub request_body: Value,
    pub response: Option<Value>,
}

impl FlowExecutionTask {
    pub fn new(
        flow: Flow,
        auth: AuthConfig,
        parameters: Value,
        request_body: Value,
    ) -> Self {
        Self {
            flow,
            execution_trace: HashMap::new(),
            current_action_index: 0,
            client: reqwest::blocking::Client::new(),
            auth,
            parameters,
            request_body,
        }
    }

    fn resolve_links(&self, action_id: &str) -> (Value, Value) {
        let mut params = json!({});
        let mut body = json!({});

        for link in &self.flow.links {
            if link.target.action_id.as_ref() == Some(&action_id.to_string()) {
                let source_value = self.get_link_source(&link.origin);
                let target_path = &link.target.field_path;

                if target_path.starts_with("parameters") {
                    self.set_json_path(&mut params, target_path, source_value);
                } else if target_path.starts_with("requestBody") {
                    self.set_json_path(&mut body, target_path, source_value);
                }
            }
        }

        (params, body)
    }

    fn get_link_source(&self, origin: &LinkOrigin) -> Value {
        match &origin.action_id {
            None => {
                // Flow parameters
                self.get_json_path(&self.parameters, &origin.field_path)
                    .unwrap_or(json!(null))
            }
            Some(action_id) => {
                // Action response
                self.execution_trace
                    .get(action_id)
                    .and_then(|trace| trace.response.as_ref())
                    .and_then(|resp| self.get_json_path(resp, &origin.field_path))
                    .unwrap_or(json!(null))
            }
        }
    }

    fn execute_action(&mut self) -> Value {
        let action = &self.flow.actions[self.current_action_index];
        let (params, body) = self.resolve_links(&action.id);

        // Mock execution for example
        json!({
            "action": action.id,
            "operation": action.operation_id,
            "status": "success"
        })
    }

    fn aggregate_response(&self) -> Value {
        // Check for flow response links
        for link in &self.flow.links {
            if link.target.action_id.is_none()
                && link.target.field_path.starts_with("responses")
            {
                let source_value = self.get_link_source(&link.origin);
                let mut result = json!({});
                self.set_json_path(&mut result, &link.target.field_path, source_value);
                return result;
            }
        }

        // Default: return last action's response
        if let Some(last_action) = self.flow.actions.last() {
            if let Some(trace) = self.execution_trace.get(&last_action.id) {
                return trace.response.clone().unwrap_or(json!({}));
            }
        }

        json!({})
    }

    // Helper methods for JSON path operations
    fn get_json_path(&self, value: &Value, path: &str) -> Option<Value> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = value;

        for part in parts {
            current = match current {
                Value::Object(obj) => obj.get(part)?,
                Value::Array(arr) => {
                    let index: usize = part.trim_matches(|c: char| !c.is_numeric()).parse().ok()?;
                    arr.get(index)?
                }
                _ => return None,
            };
        }

        Some(current.clone())
    }

    fn set_json_path(&self, value: &mut Value, path: &str, new_value: Value) {
        let parts: Vec<&str> = path.split('.').collect();
        self.set_json_path_recursive(value, &parts, new_value);
    }

    fn set_json_path_recursive(&self, value: &mut Value, path: &[&str], new_value: Value) {
        if path.is_empty() {
            *value = new_value;
            return;
        }

        let current = path[0];
        let remaining = &path[1..];

        if remaining.is_empty() {
            if let Value::Object(obj) = value {
                obj.insert(current.to_string(), new_value);
            }
            return;
        }

        if let Value::Object(obj) = value {
            if let Some(nested) = obj.get_mut(current) {
                self.set_json_path_recursive(nested, remaining, new_value);
            }
        }
    }
}

impl TaskIterator for FlowExecutionTask {
    type Ready = Value;
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        if self.current_action_index >= self.flow.actions.len() {
            let response = self.aggregate_response();
            return Some(TaskStatus::Ready(response));
        }

        // Execute current action
        let response = self.execute_action();

        // Store in trace
        let action = &self.flow.actions[self.current_action_index];
        self.execution_trace.insert(
            action.id.clone(),
            ActionTrace {
                parameters: json!({}),
                request_body: json!({}),
                response: Some(response),
            },
        );

        self.current_action_index += 1;

        Some(TaskStatus::Pending(()))
    }
}
```

### Usage

```rust
fn execute_flow_example() -> Value {
    initialize(42);

    let flow = Flow {
        id: "test_flow".to_string(),
        title: "Test Flow".to_string(),
        description: "A test flow".to_string(),
        actions: vec![
            Action {
                id: "action1".to_string(),
                source_id: "stripe".to_string(),
                operation_id: "stripe_post_products".to_string(),
                additional: HashMap::new(),
            },
        ],
        links: vec![],
        fields: Fields {
            parameters: vec![],
            request_body: None,
            responses: Responses {
                success: json!({"type": "object"}),
                example: None,
            },
            additional: HashMap::new(),
        },
        additional: None,
    };

    let auth = AuthConfig::ApiKey(ApiKeyAuth {
        key_value: "sk_test_123".to_string(),
        key_name: None,
        key_prefix: None,
    });

    let task = FlowExecutionTask::new(flow, auth, json!({}), json!({}));

    let mut result = None;
    for status in spawn().with_task(task).iter() {
        if let TaskStatus::Ready(value) = status {
            result = Some(value);
            break;
        }
    }

    result.unwrap_or(json!({"error": "No result"}))
}
```

---

## Executor Patterns

### Single-Threaded Executor

```rust
use foundation_core::valtron::single::{initialize, spawn, run_until_complete, task_iter, block_iter};

// Pattern 1: Callback-based
fn with_callback() {
    initialize(42);

    spawn()
        .with_task(my_task())
        .with_resolver(Box::new(FnReady::new(|value, _| {
            println!("Result: {:?}", value);
        })))
        .schedule()
        .unwrap();

    run_until_complete();
}

// Pattern 2: State iterator (non-blocking)
fn with_state_iter() {
    initialize(42);

    for status in task_iter(spawn().with_task(my_task())) {
        match status {
            TaskStatus::Ready(value) => println!("Ready: {:?}", value),
            TaskStatus::Pending(_) => println!("Still pending..."),
            _ => {}
        }
    }
}

// Pattern 3: Blocking iterator
fn with_block_iter() {
    initialize(42);

    for value in block_iter(spawn().with_task(my_task())) {
        println!("Value: {:?}", value);
    }
}
```

### Multi-Threaded Executor

```rust
use foundation_core::valtron::multi::{block_on, get_pool, spawn};

fn multi_threaded() {
    block_on(42, None, |pool| {
        pool.spawn()
            .with_task(my_task())
            .with_resolver(Box::new(FnReady::new(|value, _| {
                println!("Result: {:?}", value);
            })))
            .schedule()
            .unwrap();
    });
}
```

---

## Comparison: Async vs Valtron

### Async/Tokio Pattern

```rust
async fn fetch_data(url: String) -> Result<Value, Error> {
    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;
    let data = response.json().await?;
    Ok(data)
}

#[tokio::main]
async fn main() {
    let result = fetch_data("https://api.example.com/data".to_string()).await;
    println!("Result: {:?}", result);
}
```

### Valtron Pattern

```rust
struct FetchTask {
    url: String,
    executed: bool,
}

impl TaskIterator for FetchTask {
    type Ready = Value;
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        if self.executed {
            return None;
        }
        self.executed = true;

        let client = reqwest::blocking::Client::new();
        let response = client.get(&self.url).send().unwrap();
        let data: Value = response.json().unwrap();
        Some(TaskStatus::Ready(data))
    }
}

fn main() {
    initialize(42);

    let task = FetchTask {
        url: "https://api.example.com/data".to_string(),
        executed: false,
    };

    for value in block_iter(spawn().with_task(task)) {
        println!("Result: {:?}", value);
    }
}
```

---

## Best Practices

### 1. Use `NoSpawner` for Simple Tasks

```rust
type Spawner = NoSpawner;  // Don't spawn sub-tasks
```

### 2. Track State

```rust
pub struct MyTask {
    // Task configuration
    executed: bool,  // Track completion
    // ...
}
```

### 3. Return None When Complete

```rust
fn next(&mut self) -> Option<TaskStatus<...>> {
    if self.complete {
        return None;  // Signal task end
    }
    // ...
}
```

### 4. Use Appropriate Iterator

- `task_iter()` - Non-blocking, get states
- `block_iter()` - Blocking, get values
- `run_until_complete()` - Run all scheduled tasks

---

*This guide covers Valtron basics. See rust-revision.md for complete Wildcard-AI translation.*
