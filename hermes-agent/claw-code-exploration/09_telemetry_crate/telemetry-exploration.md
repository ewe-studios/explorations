# Telemetry Crate — Line-by-Line Exploration

**Crate:** `telemetry`  
**Status:** NEW in claw-code-latest (not present in original claw-code)  
**Purpose:** Structured telemetry events, session tracing, and telemetry sinks  
**Total Lines:** 526  
**Files:** `src/lib.rs` (single file crate)

---

## Table of Contents

1. [Module Overview](#module-overview)
2. [Constants and Types (Lines 1-17)](#constants-and-types)
3. [ClientIdentity (Lines 19-51)](#clientidentity)
4. [AnthropicRequestProfile (Lines 53-132)](#anthropicrequestprofile)
5. [AnalyticsEvent (Lines 134-157)](#analyticsevent)
6. [SessionTraceRecord (Lines 159-167)](#sessiontracerecord)
7. [TelemetryEvent Enum (Lines 169-203)](#telemetryevent-enum)
8. [TelemetrySink Trait (Lines 205-207)](#telemetrysink-trait)
9. [MemoryTelemetrySink (Lines 209-231)](#memorytelemetrysink)
10. [JsonlTelemetrySink (Lines 233-277)](#jsonltelemetrysink)
11. [SessionTracer (Lines 279-419)](#sessiontracer)
12. [Helper Functions (Lines 409-428)](#helper-functions)
13. [Unit Tests (Lines 430-526)](#unit-tests)
14. [Integration Points](#integration-points)

---

## Module Overview

The telemetry crate provides a complete observability layer for claw-code operations. It implements:

- **Client identity management** for API requests
- **Request profiling** with header and body construction
- **Structured event types** for HTTP lifecycle and analytics
- **Pluggable sink architecture** for event output
- **Session tracing** with sequence numbers and timestamps

This crate is foundational for debugging, monitoring, and understanding claw-code behavior.

---

## Constants and Types (Lines 1-17)

```rust
use std::fmt::{Debug, Formatter};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub const DEFAULT_ANTHROPIC_VERSION: &str = "2023-06-01";
pub const DEFAULT_APP_NAME: &str = "claude-code";
pub const DEFAULT_RUNTIME: &str = "rust";
pub const DEFAULT_AGENTIC_BETA: &str = "claude-code-20250219";
pub const DEFAULT_PROMPT_CACHING_SCOPE_BETA: &str = "prompt-caching-scope-2026-01-05";
```

### Imports Breakdown

| Line | Import | Purpose |
|------|--------|---------|
| 1-2 | `std::fmt`, `std::fs` | Debug formatting and file operations |
| 3-4 | `std::io`, `std::path` | Write trait and path handling |
| 5-6 | `std::sync::atomic`, `std::sync` | Thread-safe sequence counting, Arc<Mutex> |
| 7 | `std::time` | Timestamp generation |
| 9-10 | `serde`, `serde_json` | Serialization/deserialization |

### Default Constants

| Constant | Value | Purpose |
|----------|-------|---------|
| `DEFAULT_ANTHROPIC_VERSION` | `"2023-06-01"` | Anthropic API version header |
| `DEFAULT_APP_NAME` | `"claude-code"` | Application identifier |
| `DEFAULT_RUNTIME` | `"rust"` | Runtime identifier |
| `DEFAULT_AGENTIC_BETA` | `"claude-code-20250219"` | Agentic features beta flag |
| `DEFAULT_PROMPT_CACHING_SCOPE_BETA` | `"prompt-caching-scope-2026-01-05"` | Prompt caching beta flag |

These constants define the default configuration for API requests and client identification.

---

## ClientIdentity (Lines 19-51)

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientIdentity {
    pub app_name: String,
    pub app_version: String,
    pub runtime: String,
}
```

### Purpose
Identifies the claw-code client to upstream APIs. Used in User-Agent headers and telemetry.

### Methods

#### `new()` (Lines 26-33)
```rust
#[must_use]
pub fn new(app_name: impl Into<String>, app_version: impl Into<String>) -> Self {
    Self {
        app_name: app_name.into(),
        app_version: app_version.into(),
        runtime: DEFAULT_RUNTIME.to_string(),
    }
}
```
Creates a new ClientIdentity with the specified app name/version and default runtime.

#### `with_runtime()` (Lines 35-39)
```rust
#[must_use]
pub fn with_runtime(mut self, runtime: impl Into<String>) -> Self {
    self.runtime = runtime.into();
    self
}
```
Builder pattern method to override the default runtime.

#### `user_agent()` (Lines 41-44)
```rust
#[must_use]
pub fn user_agent() -> String {
    format!("{}/{}", self.app_name, self.app_version)
}
```
Generates the User-Agent header value (e.g., `"claude-code/0.2.1"`).

### Default Implementation (Lines 47-51)
```rust
impl Default for ClientIdentity {
    fn default() -> Self {
        Self::new(DEFAULT_APP_NAME, env!("CARGO_PKG_VERSION"))
    }
}
```
Uses compile-time package version from Cargo.toml.

---

## AnthropicRequestProfile (Lines 53-132)

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnthropicRequestProfile {
    pub anthropic_version: String,
    pub client_identity: ClientIdentity,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub betas: Vec<String>,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub extra_body: Map<String, Value>,
}
```

### Purpose
Encapsulates all configuration for Anthropic API requests including headers, betas, and body extensions.

### Methods

#### `new()` (Lines 64-75)
```rust
#[must_use]
pub fn new(client_identity: ClientIdentity) -> Self {
    Self {
        anthropic_version: DEFAULT_ANTHROPIC_VERSION.to_string(),
        client_identity,
        betas: vec![
            DEFAULT_AGENTIC_BETA.to_string(),
            DEFAULT_PROMPT_CACHING_SCOPE_BETA.to_string(),
        ],
        extra_body: Map::new(),
    }
}
```
Initializes with default API version and two beta flags enabled by default.

#### `with_beta()` (Lines 77-84)
```rust
#[must_use]
pub fn with_beta(mut self, beta: impl Into<String>) -> Self {
    let beta = beta.into();
    if !self.betas.contains(&beta) {
        self.betas.push(beta);
    }
    self
}
```
Adds a beta flag if not already present (deduplication).

#### `with_extra_body()` (Lines 86-90)
```rust
#[must_use]
pub fn with_extra_body(mut self, key: impl Into<String>, value: Value) -> Self {
    self.extra_body.insert(key.into(), value);
    self
}
```
Adds extra fields to the JSON request body.

#### `header_pairs()` (Lines 92-105)
```rust
#[must_use]
pub fn header_pairs(&self) -> Vec<(String, String)> {
    let mut headers = vec![
        (
            "anthropic-version".to_string(),
            self.anthropic_version.clone(),
        ),
        ("user-agent".to_string(), self.client_identity.user_agent()),
    ];
    if !self.betas.is_empty() {
        headers.push(("anthropic-beta".to_string(), self.betas.join(",")));
    }
    headers
}
```
**Line-by-line:**
- Line 94: Initialize headers Vec with capacity for 2-3 pairs
- Line 95-98: Add `anthropic-version` header
- Line 99: Add `user-agent` header from ClientIdentity
- Line 101-103: Conditionally add `anthropic-beta` header (comma-separated)
- Line 104: Return the header pairs

Output example:
```
[
  ("anthropic-version", "2023-06-01"),
  ("user-agent", "claude-code/0.2.1"),
  ("anthropic-beta", "claude-code-20250219,prompt-caching-scope-2026-01-05")
]
```

#### `render_json_body()` (Lines 107-125)
```rust
pub fn render_json_body<T: Serialize>(&self, request: &T) -> Result<Value, serde_json::Error> {
    let mut body = serde_json::to_value(request)?;
    let object = body.as_object_mut().ok_or_else(|| {
        serde_json::Error::io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "request body must serialize to a JSON object",
        ))
    })?;
    for (key, value) in &self.extra_body {
        object.insert(key.clone(), value.clone());
    }
    if !self.betas.is_empty() {
        object.insert(
            "betas".to_string(),
            Value::Array(self.betas.iter().cloned().map(Value::String).collect()),
        );
    }
    Ok(body)
}
```

**Line-by-line:**
- Line 108: Serialize the request to `serde_json::Value`
- Line 109-114: Extract mutable object reference, return error if not an object
- Line 115-117: Merge all `extra_body` fields into the request
- Line 118-123: Inject `betas` array if any are configured
- Line 124: Return the merged body

This method allows injecting metadata and beta flags into any request body.

### Default Implementation (Lines 128-132)
```rust
impl Default for AnthropicRequestProfile {
    fn default() -> Self {
        Self::new(ClientIdentity::default())
    }
}
```

---

## AnalyticsEvent (Lines 134-157)

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalyticsEvent {
    pub namespace: String,
    pub action: String,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub properties: Map<String, Value>,
}
```

### Purpose
Structured analytics events for tracking user interactions and system behavior.

### Methods

#### `new()` (Lines 143-150)
```rust
#[must_use]
pub fn new(namespace: impl Into<String>, action: impl Into<String>) -> Self {
    Self {
        namespace: namespace.into(),
        action: action.into(),
        properties: Map::new(),
    }
}
```

#### `with_property()` (Lines 152-156)
```rust
#[must_use]
pub fn with_property(mut self, key: impl Into<String>, value: Value) -> Self {
    self.properties.insert(key.into(), value);
    self
}
```

Example usage:
```rust
AnalyticsEvent::new("cli", "prompt_sent")
    .with_property("model", Value::String("claude-sonnet".to_string()))
```

---

## SessionTraceRecord (Lines 159-167)

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionTraceRecord {
    pub session_id: String,
    pub sequence: u64,
    pub name: String,
    pub timestamp_ms: u64,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub attributes: Map<String, Value>,
}
```

### Fields

| Field | Type | Purpose |
|-------|------|---------|
| `session_id` | `String` | Unique session identifier |
| `sequence` | `u64` | Monotonically increasing event number |
| `name` | `String` | Event name (e.g., "http_request_started") |
| `timestamp_ms` | `u64` | Unix timestamp in milliseconds |
| `attributes` | `Map<String, Value>` | Event-specific metadata |

The sequence number ensures events can be ordered even if timestamps drift.

---

## TelemetryEvent Enum (Lines 169-203)

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TelemetryEvent {
    HttpRequestStarted {
        session_id: String,
        attempt: u32,
        method: String,
        path: String,
        #[serde(default, skip_serializing_if = "Map::is_empty")]
        attributes: Map<String, Value>,
    },
    HttpRequestSucceeded {
        session_id: String,
        attempt: u32,
        method: String,
        path: String,
        status: u16,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        #[serde(default, skip_serializing_if = "Map::is_empty")]
        attributes: Map<String, Value>,
    },
    HttpRequestFailed {
        session_id: String,
        attempt: u32,
        method: String,
        path: String,
        error: String,
        retryable: bool,
        #[serde(default, skip_serializing_if = "Map::is_empty")]
        attributes: Map<String, Value>,
    },
    Analytics(AnalyticsEvent),
    SessionTrace(SessionTraceRecord),
}
```

### Variant Breakdown

#### `HttpRequestStarted` (Lines 172-179)
Emitted when an HTTP request is initiated. Tracks retry attempts.

#### `HttpRequestSucceeded` (Lines 180-190)
Emitted on successful response. Includes:
- HTTP status code
- Upstream request ID (for debugging with Anthropic)

#### `HttpRequestFailed` (Lines 191-200)
Emitted on failure. Key field:
- `retryable: bool` - Indicates if the error should trigger a retry

#### `Analytics` (Line 201)
Wraps an `AnalyticsEvent` for user interaction tracking.

#### `SessionTrace` (Line 202)
Wraps a `SessionTraceRecord` for ordered session events.

### Serialization Format
The `#[serde(tag = "type", rename_all = "snake_case")]` attribute produces:
```json
{
  "type": "http_request_succeeded",
  "session_id": "session-123",
  "attempt": 1,
  "method": "POST",
  "path": "/v1/messages",
  "status": 200,
  "request_id": "req_abc123"
}
```

---

## TelemetrySink Trait (Lines 205-207)

```rust
pub trait TelemetrySink: Send + Sync {
    fn record(&self, event: TelemetryEvent);
}
```

### Purpose
Pluggable output backend for telemetry events. The `Send + Sync` bounds allow sharing across threads.

### Implementations
1. `MemoryTelemetrySink` - In-memory buffer for testing
2. `JsonlTelemetrySink` - JSONL file output for production

---

## MemoryTelemetrySink (Lines 209-231)

```rust
#[derive(Default)]
pub struct MemoryTelemetrySink {
    events: Mutex<Vec<TelemetryEvent>>,
}
```

### Implementation

#### `events()` (Lines 214-221)
```rust
#[must_use]
pub fn events(&self) -> Vec<TelemetryEvent> {
    self.events
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
}
```
Returns a clone of all recorded events. Handles poisoned locks gracefully.

#### `record()` (Lines 224-230)
```rust
impl TelemetrySink for MemoryTelemetrySink {
    fn record(&self, event: TelemetryEvent) {
        self.events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(event);
    }
}
```
Thread-safe append to the events vector.

### Use Case
Used in unit tests and integration tests to verify telemetry is emitted correctly.

---

## JsonlTelemetrySink (Lines 233-277)

```rust
pub struct JsonlTelemetrySink {
    path: PathBuf,
    file: Mutex<File>,
}
```

### Implementation

#### `Debug` (Lines 238-244)
```rust
impl Debug for JsonlTelemetrySink {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JsonlTelemetrySink")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}
```
Custom debug impl to avoid logging file handle.

#### `new()` (Lines 246-257)
```rust
pub fn new(path: impl AsRef<Path>) -> Result<Self, std::io::Error> {
    let path = path.as_ref().to_path_buf();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = OpenOptions::new().create(true).append(true).open(&path)?;
    Ok(Self {
        path,
        file: Mutex::new(file),
    })
}
```

**Line-by-line:**
- Line 248: Convert to owned PathBuf
- Line 249-251: Create parent directories if needed
- Line 252: Open file in create+append mode
- Line 253-256: Construct the sink with mutex-protected file

#### `path()` (Lines 259-262)
```rust
#[must_use]
pub fn path(&self) -> &Path {
    &self.path
}
```
Returns the file path for reference.

#### `record()` (Lines 265-276)
```rust
impl TelemetrySink for JsonlTelemetrySink {
    fn record(&self, event: TelemetryEvent) {
        let Ok(line) = serde_json::to_string(&event) else {
            return;
        };
        let mut file = self
            .file
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _ = writeln!(file, "{line}");
        let _ = file.flush();
    }
}
```

**Line-by-line:**
- Line 267-269: Serialize event to JSON string, silently skip on error
- Line 270-273: Acquire file lock with poison handling
- Line 274: Write JSON line with newline
- Line 275: Flush to ensure durability

### Output Format
Each event is one line of JSON:
```
{"type":"analytics","namespace":"cli","action":"turn_completed","properties":{"ok":true}}
{"type":"http_request_started","session_id":"session-123","attempt":1,"method":"POST","path":"/v1/messages"}
```

---

## SessionTracer (Lines 279-419)

```rust
#[derive(Clone)]
pub struct SessionTracer {
    session_id: String,
    sequence: Arc<AtomicU64>,
    sink: Arc<dyn TelemetrySink>,
}
```

### Purpose
High-level facade for recording telemetry events within a session. Manages sequence numbers and dual-writing to both raw events and session traces.

### Implementation

#### `Debug` (Lines 286-292)
```rust
impl Debug for SessionTracer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionTracer")
            .field("session_id", &self.session_id)
            .finish_non_exhaustive()
    }
}
```

#### `new()` (Lines 295-302)
```rust
#[must_use]
pub fn new(session_id: impl Into<String>, sink: Arc<dyn TelemetrySink>) -> Self {
    Self {
        session_id: session_id.into(),
        sequence: Arc::new(AtomicU64::new(0)),
        sink,
    }
}
```
Initializes sequence counter at 0.

#### `session_id()` (Lines 304-307)
```rust
#[must_use]
pub fn session_id(&self) -> &str {
    &self.session_id
}
```
Accessor for the session identifier.

#### `record()` (Lines 309-318)
```rust
pub fn record(&self, name: impl Into<String>, attributes: Map<String, Value>) {
    let record = SessionTraceRecord {
        session_id: self.session_id.clone(),
        sequence: self.sequence.fetch_add(1, Ordering::Relaxed),
        name: name.into(),
        timestamp_ms: current_timestamp_ms(),
        attributes,
    };
    self.sink.record(TelemetryEvent::SessionTrace(record));
}
```

**Line-by-line:**
- Line 310-316: Construct SessionTraceRecord
- Line 312: Atomically increment sequence (Relaxed ordering is sufficient for tracing)
- Line 314: Get current Unix timestamp in ms
- Line 317: Record as SessionTrace event

#### `record_http_request_started()` (Lines 320-340)
```rust
pub fn record_http_request_started(
    &self,
    attempt: u32,
    method: impl Into<String>,
    path: impl Into<String>,
    attributes: Map<String, Value>,
) {
    let method = method.into();
    let path = path.into();
    self.sink.record(TelemetryEvent::HttpRequestStarted {
        session_id: self.session_id.clone(),
        attempt,
        method: method.clone(),
        path: path.clone(),
        attributes: attributes.clone(),
    });
    self.record(
        "http_request_started",
        merge_trace_fields(method, path, attempt, attributes),
    );
}
```

**Key behavior:**
- Line 329-335: Record the strongly-typed `HttpRequestStarted` event
- Line 336-339: Also record a `SessionTrace` with merged fields

This dual-write pattern ensures both structured querying and session-ordered tracing.

#### `record_http_request_succeeded()` (Lines 342-368)
```rust
pub fn record_http_request_succeeded(
    &self,
    attempt: u32,
    method: impl Into<String>,
    path: impl Into<String>,
    status: u16,
    request_id: Option<String>,
    attributes: Map<String, Value>,
) {
    let method = method.into();
    let path = path.into();
    self.sink.record(TelemetryEvent::HttpRequestSucceeded {
        session_id: self.session_id.clone(),
        attempt,
        method: method.clone(),
        path: path.clone(),
        status,
        request_id: request_id.clone(),
        attributes: attributes.clone(),
    });
    let mut trace_attributes = merge_trace_fields(method, path, attempt, attributes);
    trace_attributes.insert("status".to_string(), Value::from(status));
    if let Some(request_id) = request_id {
        trace_attributes.insert("request_id".to_string(), Value::String(request_id));
    }
    self.record("http_request_succeeded", trace_attributes);
}
```

**Line-by-line:**
- Line 353-360: Record strongly-typed event
- Line 362: Start with base trace fields
- Line 363: Add status code
- Line 364-366: Conditionally add request_id
- Line 367: Record SessionTrace

#### `record_http_request_failed()` (Lines 370-395)
```rust
pub fn record_http_request_failed(
    &self,
    attempt: u32,
    method: impl Into<String>,
    path: impl Into<String>,
    error: impl Into<String>,
    retryable: bool,
    attributes: Map<String, Value>,
) {
    let method = method.into();
    let path = path.into();
    let error = error.into();
    self.sink.record(TelemetryEvent::HttpRequestFailed {
        session_id: self.session_id.clone(),
        attempt,
        method: method.clone(),
        path: path.clone(),
        error: error.clone(),
        retryable,
        attributes: attributes.clone(),
    });
    let mut trace_attributes = merge_trace_fields(method, path, attempt, attributes);
    trace_attributes.insert("error".to_string(), Value::String(error));
    trace_attributes.insert("retryable".to_string(), Value::Bool(retryable));
    self.record("http_request_failed", trace_attributes);
}
```

Similar dual-write pattern with error details and retryable flag.

#### `record_analytics()` (Lines 397-406)
```rust
pub fn record_analytics(&self, event: AnalyticsEvent) {
    let mut attributes = event.properties.clone();
    attributes.insert(
        "namespace".to_string(),
        Value::String(event.namespace.clone()),
    );
    attributes.insert("action".to_string(), Value::String(event.action.clone()));
    self.sink.record(TelemetryEvent::Analytics(event));
    self.record("analytics", attributes);
}
```

Extracts namespace and action into trace attributes for querying.

---

## Helper Functions (Lines 409-428)

### `merge_trace_fields()` (Lines 409-419)
```rust
fn merge_trace_fields(
    method: String,
    path: String,
    attempt: u32,
    mut attributes: Map<String, Value>,
) -> Map<String, Value> {
    attributes.insert("method".to_string(), Value::String(method));
    attributes.insert("path".to_string(), Value::String(path));
    attributes.insert("attempt".to_string(), Value::from(attempt));
    attributes
}
```
Injects HTTP request fields into trace attributes.

### `current_timestamp_ms()` (Lines 421-428)
```rust
fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}
```
Gets current Unix timestamp in milliseconds with fallback on error.

---

## Unit Tests (Lines 430-526)

### Test 1: `request_profile_emits_headers_and_merges_body()` (Lines 434-473)

```rust
#[test]
fn request_profile_emits_headers_and_merges_body() {
    let profile = AnthropicRequestProfile::new(
        ClientIdentity::new("claude-code", "1.2.3").with_runtime("rust-cli"),
    )
    .with_beta("tools-2026-04-01")
    .with_extra_body("metadata", serde_json::json!({"source": "test"}));

    assert_eq!(
        profile.header_pairs(),
        vec![
            (
                "anthropic-version".to_string(),
                DEFAULT_ANTHROPIC_VERSION.to_string()
            ),
            ("user-agent".to_string(), "claude-code/1.2.3".to_string()),
            (
                "anthropic-beta".to_string(),
                "claude-code-20250219,prompt-caching-scope-2026-01-05,tools-2026-04-01"
                    .to_string(),
            ),
        ]
    );

    let body = profile
        .render_json_body(&serde_json::json!({"model": "claude-sonnet"}))
        .expect("body should serialize");
    assert_eq!(
        body["metadata"]["source"],
        Value::String("test".to_string())
    );
    assert_eq!(
        body["betas"],
        serde_json::json!([
            "claude-code-20250219",
            "prompt-caching-scope-2026-01-05",
            "tools-2026-04-01"
        ])
    );
}
```

**Verifies:**
- Header pairs include version, user-agent, and beta flags
- Extra body fields are merged
- Betas array is injected into request body

### Test 2: `session_tracer_records_structured_events_and_trace_sequence()` (Lines 475-508)

```rust
#[test]
fn session_tracer_records_structured_events_and_trace_sequence() {
    let sink = Arc::new(MemoryTelemetrySink::default());
    let tracer = SessionTracer::new("session-123", sink.clone());

    tracer.record_http_request_started(1, "POST", "/v1/messages", Map::new());
    tracer.record_analytics(
        AnalyticsEvent::new("cli", "prompt_sent")
            .with_property("model", Value::String("claude-opus".to_string())),
    );

    let events = sink.events();
    assert!(matches!(
        &events[0],
        TelemetryEvent::HttpRequestStarted {
            session_id,
            attempt: 1,
            method,
            path,
            ..
        } if session_id == "session-123" && method == "POST" && path == "/v1/messages"
    ));
    assert!(matches!(
        &events[1],
        TelemetryEvent::SessionTrace(SessionTraceRecord { sequence: 0, name, .. })
        if name == "http_request_started"
    ));
    assert!(matches!(&events[2], TelemetryEvent::Analytics(_)));
    assert!(matches!(
        &events[3],
        TelemetryEvent::SessionTrace(SessionTraceRecord { sequence: 1, name, .. })
        if name == "analytics"
    ));
}
```

**Verifies:**
- Dual-write pattern produces 4 events from 2 calls
- Sequence numbers are monotonic (0, 1)
- Event types match expected variants

### Test 3: `jsonl_sink_persists_events()` (Lines 510-525)

```rust
#[test]
fn jsonl_sink_persists_events() {
    let path =
        std::env::temp_dir().join(format!("telemetry-jsonl-{}.log", current_timestamp_ms()));
    let sink = JsonlTelemetrySink::new(&path).expect("sink should create file");

    sink.record(TelemetryEvent::Analytics(
        AnalyticsEvent::new("cli", "turn_completed").with_property("ok", Value::Bool(true)),
    ));

    let contents = std::fs::read_to_string(&path).expect("telemetry log should be readable");
    assert!(contents.contains("\"type\":\"analytics\""));
    assert!(contents.contains("\"action\":\"turn_completed\""));

    let _ = std::fs::remove_file(path);
}
```

**Verifies:**
- File is created automatically
- Events are serialized as JSONL
- Cleanup removes temp file

---

## Integration Points

### Upstream Dependencies
| Crate | Usage |
|-------|-------|
| `api` | Uses `AnthropicRequestProfile` for API request construction |
| `runtime` | Uses `SessionTracer` for operation tracing |

### Downstream Dependents
| Crate | How it uses telemetry |
|-------|----------------------|
| `rusty-claude-cli` | Records CLI session events |
| `runtime` | Traces HTTP requests, MCP lifecycle |

### File Output Location
JSONL telemetry files are written to:
```
~/.claude/telemetry/{session-id}.jsonl
```

---

## Summary

The telemetry crate is a compact but complete observability solution:

| Component | Lines | Purpose |
|-----------|-------|---------|
| ClientIdentity | 33 | Client identification |
| AnthropicRequestProfile | 80 | Request configuration |
| AnalyticsEvent | 24 | User analytics |
| SessionTraceRecord | 9 | Ordered tracing |
| TelemetryEvent | 35 | Event taxonomy |
| TelemetrySink | 3 | Pluggable output |
| MemoryTelemetrySink | 23 | Testing |
| JsonlTelemetrySink | 45 | Production file output |
| SessionTracer | 141 | High-level recording |
| Tests | 97 | Coverage |

**Key design patterns:**
1. Dual-write (structured + trace) for flexible querying
2. Builder pattern for configuration
3. Trait-based sink abstraction for testability
4. Atomic sequence numbers for ordering guarantees
