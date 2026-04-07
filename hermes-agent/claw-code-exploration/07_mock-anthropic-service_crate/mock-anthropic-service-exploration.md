# Mock Anthropic Service Crate — Line-by-Line Exploration

**Crate:** `mock-anthropic-service`  
**Status:** NEW in claw-code-latest (not present in original claw-code)  
**Purpose:** Deterministic mock Anthropic API server for parity testing  
**Total Lines:** 1,157 (lib.rs: 1,123 + main.rs: 34)  
**Files:** `src/lib.rs`, `src/main.rs`

---

## Table of Contents

1. [Module Overview](#module-overview)
2. [Constants and Types (Lines 1-24)](#constants-and-types)
3. [MockAnthropicService (Lines 26-87)](#mockanthropicservice)
4. [Scenario Enum (Lines 89-140)](#scenario-enum)
5. [Connection Handler (Lines 142-254)](#connection-handler)
6. [Scenario Detection (Lines 244-308)](#scenario-detection)
7. [Response Builder (Lines 310-638)](#response-builder)
8. [HTTP Response Helpers (Lines 640-762)](#http-response-helpers)
9. [SSE Stream Builders (Lines 764-1048)](#sse-stream-builders)
10. [Extraction Helpers (Lines 1067-1123)](#extraction-helpers)
11. [Main Binary (main.rs)](#main-binary)
12. [Integration Points](#integration-points)

---

## Module Overview

The mock-anthropic-service crate provides a **deterministic, local HTTP server** that mimics the Anthropic API. This enables:

- **Parity testing** between claw-code and upstream Claude Code
- **Zero-cost integration tests** without API calls
- **Reproducible scenarios** for all tool types
- **Request capture** for validation

The service uses a **scenario-based** approach where test cases embed a scenario identifier in the prompt, and the mock service responds with predefined behavior.

---

## Constants and Types (Lines 1-24)

```rust
use std::collections::HashMap;
use std::io;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use api::{InputContentBlock, MessageRequest, MessageResponse, OutputContentBlock, Usage};
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::{oneshot, Mutex};
use tokio::task::JoinHandle;

pub const SCENARIO_PREFIX: &str = "PARITY_SCENARIO:";
pub const DEFAULT_MODEL: &str = "claude-sonnet-4-6";
```

### Imports Breakdown

| Line | Import | Purpose |
|------|--------|---------|
| 1-4 | `std::collections`, `std::io`, `std::sync`, `std::time` | Standard utilities |
| 6 | `api::` | Shared types from api crate (MessageRequest, MessageResponse) |
| 7 | `serde_json` | JSON parsing and generation |
| 8-11 | `tokio::` | Async networking, synchronization |

### Public Constants

| Constant | Value | Purpose |
|----------|-------|---------|
| `SCENARIO_PREFIX` | `"PARITY_SCENARIO:"` | Token prefix to detect test scenario |
| `DEFAULT_MODEL` | `"claude-sonnet-4-6"` | Default model for mock responses |

### CapturedRequest (Lines 16-24)

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedRequest {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub scenario: String,
    pub stream: bool,
    pub raw_body: String,
}
```

**Fields:**

| Field | Type | Purpose |
|-------|------|---------|
| `method` | `String` | HTTP method (POST) |
| `path` | `String` | Request path (/v1/messages) |
| `headers` | `HashMap` | Lowercase header map |
| `scenario` | `String` | Detected scenario name |
| `stream` | `bool` | Whether streaming was requested |
| `raw_body` | `String` | Original JSON body for inspection |

Used in tests to verify the request format sent by claw-code.

---

## MockAnthropicService (Lines 26-87)

```rust
pub struct MockAnthropicService {
    base_url: String,
    requests: Arc<Mutex<Vec<CapturedRequest>>>,
    shutdown: Option<oneshot::Sender<()>>,
    join_handle: JoinHandle<()>,
}
```

### Purpose
Represents a running mock HTTP server. Manages lifecycle (spawn/shutdown) and request capture.

### Implementation

#### `spawn()` (Lines 34-36)
```rust
pub async fn spawn() -> io::Result<Self> {
    Self::spawn_on("127.0.0.1:0").await
}
```
Convenience method to spawn on any available port.

#### `spawn_on()` (Lines 38-68)
```rust
pub async fn spawn_on(bind_addr: &str) -> io::Result<Self> {
    let listener = TcpListener::bind(bind_addr).await?;
    let address = listener.local_addr()?;
    let requests = Arc::new(Mutex::new(Vec::new()));
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
    let request_state = Arc::clone(&requests);

    let join_handle = tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = &mut shutdown_rx => break,
                accepted = listener.accept() => {
                    let Ok((socket, _)) = accepted else {
                        break;
                    };
                    let request_state = Arc::clone(&request_state);
                    tokio::spawn(async move {
                        let _ = handle_connection(socket, request_state).await;
                    });
                }
            }
        }
    });

    Ok(Self {
        base_url: format!("http://{address}"),
        requests,
        shutdown: Some(shutdown_tx),
        join_handle,
    })
}
```

**Line-by-line:**

- Line 39: Bind TCP listener (port 0 = OS assigns ephemeral port)
- Line 40: Get the assigned address
- Line 41: Create shared request storage (Arc<Mutex<Vec>>)
- Line 42: Create shutdown channel
- Line 43: Clone Arc for spawned task
- Line 45-60: Spawn server loop
  - Line 47: `tokio::select!` for shutdown signal OR incoming connection
  - Line 48: Break on shutdown signal
  - Line 49-57: Accept connection and spawn handler
  - Line 53-54: Clone request_state for each connection handler
- Line 62-67: Construct service struct

**Key design:**
- Each connection spawns a new task (concurrent request handling)
- Graceful shutdown via oneshot channel
- Shared state via Arc<Mutex>

#### `base_url()` (Lines 70-73)
```rust
#[must_use]
pub fn base_url(&self) -> String {
    self.base_url.clone()
}
```
Returns the mock server URL (e.g., `http://127.0.0.1:54321`).

#### `captured_requests()` (Lines 75-77)
```rust
pub async fn captured_requests(&self) -> Vec<CapturedRequest> {
    self.requests.lock().await.clone()
}
```
Returns all captured requests (for test assertions).

### Drop Implementation (Lines 80-87)
```rust
impl Drop for MockAnthropicService {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        self.join_handle.abort();
    }
}
```
Ensures clean shutdown when service goes out of scope.

---

## Scenario Enum (Lines 89-140)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scenario {
    StreamingText,
    ReadFileRoundtrip,
    GrepChunkAssembly,
    WriteFileAllowed,
    WriteFileDenied,
    MultiToolTurnRoundtrip,
    BashStdoutRoundtrip,
    BashPermissionPromptApproved,
    BashPermissionPromptDenied,
    PluginToolRoundtrip,
    AutoCompactTriggered,
    TokenCostReporting,
}
```

### Purpose
Defines 12 parity testing scenarios covering all major claw-code interactions.

### `parse()` (Lines 105-122)
```rust
fn parse(value: &str) -> Option<Self> {
    match value.trim() {
        "streaming_text" => Some(Self::StreamingText),
        "read_file_roundtrip" => Some(Self::ReadFileRoundtrip),
        "grep_chunk_assembly" => Some(Self::GrepChunkAssembly),
        "write_file_allowed" => Some(Self::WriteFileAllowed),
        "write_file_denied" => Some(Self::WriteFileDenied),
        "multi_tool_turn_roundtrip" => Some(Self::MultiToolTurnRoundtrip),
        "bash_stdout_roundtrip" => Some(Self::BashStdoutRoundtrip),
        "bash_permission_prompt_approved" => Some(Self::BashPermissionPromptApproved),
        "bash_permission_prompt_denied" => Some(Self::BashPermissionPromptDenied),
        "plugin_tool_roundtrip" => Some(Self::PluginToolRoundtrip),
        "auto_compact_triggered" => Some(Self::AutoCompactTriggered),
        "token_cost_reporting" => Some(Self::TokenCostReporting),
        _ => None,
    }
}
```

### `name()` (Lines 124-139)
```rust
fn name(self) -> &'static str {
    match self {
        Self::StreamingText => "streaming_text",
        Self::ReadFileRoundtrip => "read_file_roundtrip",
        // ... etc
    }
}
```
Returns the canonical scenario name string.

### Scenario Descriptions

| Scenario | Tests |
|----------|-------|
| `StreamingText` | Basic text streaming via SSE |
| `ReadFileRoundtrip` | File read tool request/response |
| `GrepChunkAssembly` | Grep search with count output |
| `WriteFileAllowed` | File write within permissions |
| `WriteFileDenied` | File write blocked by permissions |
| `MultiToolTurnRoundtrip` | Multiple tools in single turn |
| `BashStdoutRoundtrip` | Bash execution with stdout capture |
| `BashPermissionPromptApproved` | Bash requires and gets approval |
| `BashPermissionPromptDenied` | Bash requires but denied approval |
| `PluginToolRoundtrip` | External plugin tool invocation |
| `AutoCompactTriggered` | Session auto-compaction trigger |
| `TokenCostReporting` | Token usage reporting |

---

## Connection Handler (Lines 142-254)

### `handle_connection()` (Lines 142-164)
```rust
async fn handle_connection(
    mut socket: tokio::net::TcpStream,
    requests: Arc<Mutex<Vec<CapturedRequest>>>,
) -> io::Result<()> {
    let (method, path, headers, raw_body) = read_http_request(&mut socket).await?;
    let request: MessageRequest = serde_json::from_str(&raw_body)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
    let scenario = detect_scenario(&request)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing parity scenario"))?;

    requests.lock().await.push(CapturedRequest {
        method,
        path,
        headers,
        scenario: scenario.name().to_string(),
        stream: request.stream,
        raw_body,
    });

    let response = build_http_response(&request, scenario);
    socket.write_all(response.as_bytes()).await?;
    Ok(())
}
```

**Line-by-line:**

- Line 146: Parse raw HTTP request
- Line 147-148: Deserialize JSON body to MessageRequest
- Line 149-150: Detect scenario from message content
- Line 152-159: Capture request for later inspection
- Line 161: Build appropriate response
- Line 162: Write HTTP response to socket

### `read_http_request()` (Lines 166-238)

```rust
async fn read_http_request(
    socket: &mut tokio::net::TcpStream,
) -> io::Result<(String, String, HashMap<String, String>, String)> {
    let mut buffer = Vec::new();
    let mut header_end = None;

    loop {
        let mut chunk = [0_u8; 1024];
        let read = socket.read(&mut chunk).await?;
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);
        if let Some(position) = find_header_end(&buffer) {
            header_end = Some(position);
            break;
        }
    }

    let header_end = header_end
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "missing http headers"))?;
    let (header_bytes, remaining) = buffer.split_at(header_end);
    let header_text = String::from_utf8(header_bytes.to_vec())
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
    let mut lines = header_text.split("\r\n");
    let request_line = lines
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing request line"))?;
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing method"))?
        .to_string();
    let path = request_parts
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing path"))?
        .to_string();

    let mut headers = HashMap::new();
    let mut content_length = 0_usize;
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let (name, value) = line.split_once(':').ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "malformed http header line")
        })?;
        let value = value.trim().to_string();
        if name.eq_ignore_ascii_case("content-length") {
            content_length = value.parse().map_err(|error| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("invalid content-length: {error}"),
                )
            })?;
        }
        headers.insert(name.to_ascii_lowercase(), value);
    }

    let mut body = remaining[4..].to_vec();
    while body.len() < content_length {
        let mut chunk = vec![0_u8; content_length - body.len()];
        let read = socket.read(&mut chunk).await?;
        if read == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..read]);
    }

    let body = String::from_utf8(body)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
    Ok((method, path, headers, body))
}
```

**Detailed breakdown:**

#### Header Reading (Lines 169-183)
- Line 169-170: Initialize buffer and header end marker
- Line 172-183: Read chunks until `\r\n\r\n` found
- Line 173-177: Read up to 1024 bytes, break on EOF
- Line 178-182: Check for header terminator

#### Header Parsing (Lines 185-223)
- Line 185-186: Ensure headers were found
- Line 187: Split headers from body
- Line 188-189: Convert header bytes to string
- Line 190: Split by CRLF
- Line 191-193: Extract request line
- Line 194-202: Parse method and path from `METHOD /path HTTP/1.1`
- Line 204-223: Parse header lines
  - Line 206-209: Skip empty lines
  - Line 210-212: Split `Name: Value`
  - Line 213-220: Extract content-length
  - Line 222: Store headers as lowercase

#### Body Reading (Lines 225-237)
- Line 225: Skip `\r\n\r\n` (4 bytes)
- Line 226-233: Read remaining body bytes per content-length
- Line 235-236: Convert body to UTF-8 string

### `find_header_end()` (Lines 240-242)
```rust
fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}
```
Finds the position of `\r\n\r\n` (HTTP header terminator).

---

## Scenario Detection (Lines 244-308)

### `detect_scenario()` (Lines 244-254)
```rust
fn detect_scenario(request: &MessageRequest) -> Option<Scenario> {
    request.messages.iter().rev().find_map(|message| {
        message.content.iter().rev().find_map(|block| match block {
            InputContentBlock::Text { text } => text
                .split_whitespace()
                .find_map(|token| token.strip_prefix(SCENARIO_PREFIX))
                .and_then(Scenario::parse),
            _ => None,
        })
    })
}
```

**How it works:**
1. Iterate messages in reverse (most recent first)
2. Iterate content blocks in reverse
3. Look for Text blocks
4. Find token starting with `PARITY_SCENARIO:`
5. Parse to Scenario enum

**Example prompt:**
```
PARITY_SCENARIO:read_file_roundtrip

Please read the fixture.txt file.
```

### `latest_tool_result()` (Lines 256-265)
```rust
fn latest_tool_result(request: &MessageRequest) -> Option<(String, bool)> {
    request.messages.iter().rev().find_map(|message| {
        message.content.iter().rev().find_map(|block| match block {
            InputContentBlock::ToolResult {
                content, is_error, ..
            } => Some((flatten_tool_result_content(content), *is_error)),
            _ => None,
        })
    })
}
```
Extracts the most recent tool result text and error flag.

### `tool_results_by_name()` (Lines 267-297)
```rust
fn tool_results_by_name(request: &MessageRequest) -> HashMap<String, (String, bool)> {
    let mut tool_names_by_id = HashMap::new();
    for message in &request.messages {
        for block in &message.content {
            if let InputContentBlock::ToolUse { id, name, .. } = block {
                tool_names_by_id.insert(id.clone(), name.clone());
            }
        }
    }

    let mut results = HashMap::new();
    for message in request.messages.iter().rev() {
        for block in message.content.iter().rev() {
            if let InputContentBlock::ToolResult {
                tool_use_id,
                content,
                is_error,
            } = block
            {
                let tool_name = tool_names_by_id
                    .get(tool_use_id)
                    .cloned()
                    .unwrap_or_else(|| tool_use_id.clone());
                results
                    .entry(tool_name)
                    .or_insert_with(|| (flatten_tool_result_content(content), *is_error));
            }
        }
    }
    results
}
```

**Purpose:** Maps tool names to their results for multi-tool scenarios.

**Algorithm:**
1. First pass: Build id→name mapping from ToolUse blocks
2. Second pass: Match ToolResult blocks to names

### `flatten_tool_result_content()` (Lines 299-308)
```rust
fn flatten_tool_result_content(content: &[api::ToolResultContentBlock]) -> String {
    content
        .iter()
        .map(|block| match block {
            api::ToolResultContentBlock::Text { text } => text.clone(),
            api::ToolResultContentBlock::Json { value } => value.to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}
```
Converts tool result content blocks to a single string.

---

## Response Builder (Lines 310-638)

### `build_http_response()` (Lines 310-330)
```rust
#[allow(clippy::too_many_lines)]
fn build_http_response(request: &MessageRequest, scenario: Scenario) -> String {
    let response = if request.stream {
        let body = build_stream_body(request, scenario);
        return http_response(
            "200 OK",
            "text/event-stream",
            &body,
            &[("x-request-id", request_id_for(scenario))],
        );
    } else {
        build_message_response(request, scenario)
    };

    http_response(
        "200 OK",
        "application/json",
        &serde_json::to_string(&response).expect("message response should serialize"),
        &[("request-id", request_id_for(scenario))],
    )
}
```

**Line-by-line:**
- Line 312-319: For streaming requests, build SSE body and return immediately
- Line 314-318: Returns early to avoid double response
- Line 321: For non-streaming, build MessageResponse
- Line 324-329: Serialize to JSON and wrap in HTTP response

### `build_stream_body()` (Lines 332-468)

This is the **core scenario logic** - 136 lines of pattern matching.

```rust
#[allow(clippy::too_many_lines)]
fn build_stream_body(request: &MessageRequest, scenario: Scenario) -> String {
    match scenario {
        Scenario::StreamingText => streaming_text_sse(),
        Scenario::ReadFileRoundtrip => match latest_tool_result(request) {
            Some((tool_output, _)) => final_text_sse(&format!(
                "read_file roundtrip complete: {}",
                extract_read_content(&tool_output)
            )),
            None => tool_use_sse(
                "toolu_read_fixture",
                "read_file",
                &[r#"{"path":"fixture.txt"}"#],
            ),
        },
        // ... (10 more scenarios)
    }
}
```

**Pattern for each scenario:**
1. Check if there's a tool result (follow-up turn)
2. If yes: Return final text response
3. If no: Return tool use request

#### StreamingText (Line 335)
```rust
Scenario::StreamingText => streaming_text_sse(),
```
Simple text streaming test.

#### ReadFileRoundtrip (Lines 336-346)
```rust
Scenario::ReadFileRoundtrip => match latest_tool_result(request) {
    Some((tool_output, _)) => final_text_sse(&format!(
        "read_file roundtrip complete: {}",
        extract_read_content(&tool_output)
    )),
    None => tool_use_sse(
        "toolu_read_fixture",
        "read_file",
        &[r#"{"path":"fixture.txt"}"#],
    ),
},
```

**Two-turn flow:**
1. Turn 1: Mock responds with `read_file` tool use
2. Turn 2: claw-code sends tool result, mock responds with final text

#### GrepChunkAssembly (Lines 347-361)
Tests grep search with chunked JSON assembly.

#### WriteFileAllowed/WriteFileDenied (Lines 362-382)
Tests permission system - one succeeds, one fails.

#### MultiToolTurnRoundtrip (Lines 383-411)
Tests parallel tool execution.

#### BashStdoutRoundtrip (Lines 412-422)
Tests bash tool with stdout extraction.

#### BashPermissionPromptApproved/Denied (Lines 423-449)
Tests permission prompt flow.

#### PluginToolRoundtrip (Lines 450-460)
Tests plugin tool invocation.

#### AutoCompactTriggered/TokenCostReporting (Lines 461-466)
Tests session management and token reporting.

### `build_message_response()` (Lines 470-638)

Same scenario logic as `build_stream_body` but returns `MessageResponse` objects for non-streaming requests.

---

## HTTP Response Helpers (Lines 640-762)

### `request_id_for()` (Lines 640-655)
```rust
fn request_id_for(scenario: Scenario) -> &'static str {
    match scenario {
        Scenario::StreamingText => "req_streaming_text",
        // ...
    }
}
```
Returns deterministic request ID for each scenario.

### `http_response()` (Lines 657-667)
```rust
fn http_response(status: &str, content_type: &str, body: &str, headers: &[(&str, &str)]) -> String {
    let mut extra_headers = String::new();
    for (name, value) in headers {
        use std::fmt::Write as _;
        write!(&mut extra_headers, "{name}: {value}\r\n").expect("header write should succeed");
    }
    format!(
        "HTTP/1.1 {status}\r\ncontent-type: {content_type}\r\n{extra_headers}content-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    )
}
```

**Output format:**
```
HTTP/1.1 200 OK
content-type: application/json
request-id: req_read_file_roundtrip
content-length: 256
connection: close

{"id":"msg_...","type":"message",...}
```

### `text_message_response()` (Lines 669-688)
```rust
fn text_message_response(id: &str, text: &str) -> MessageResponse {
    MessageResponse {
        id: id.to_string(),
        kind: "message".to_string(),
        role: "assistant".to_string(),
        content: vec![OutputContentBlock::Text {
            text: text.to_string(),
        }],
        model: DEFAULT_MODEL.to_string(),
        stop_reason: Some("end_turn".to_string()),
        stop_sequence: None,
        usage: Usage {
            input_tokens: 10,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
            output_tokens: 6,
        },
        request_id: None,
    }
}
```

### `text_message_response_with_usage()` (Lines 690-714)
Same as above but with configurable token counts.

### `tool_message_response()` (Lines 716-730)
Creates a response with a single tool use.

### `tool_message_response_many()` (Lines 738-762)
Creates a response with multiple parallel tool uses.

### `ToolUseMessage` (Lines 732-736)
Helper struct for building tool use responses.

---

## SSE Stream Builders (Lines 764-1048)

### `streaming_text_sse()` (Lines 764-829)

```rust
fn streaming_text_sse() -> String {
    let mut body = String::new();
    append_sse(&mut body, "message_start", json!({...}));
    append_sse(&mut body, "content_block_start", json!({...}));
    append_sse(&mut body, "content_block_delta", json!({"delta": {"text": "Mock streaming "}}));
    append_sse(&mut body, "content_block_delta", json!({"delta": {"text": "says hello..."}}));
    append_sse(&mut body, "content_block_stop", json!({...}));
    append_sse(&mut body, "message_delta", json!({...}));
    append_sse(&mut body, "message_stop", json!({...}));
    body
}
```

**SSE event sequence:**
1. `message_start` - Initialize message with usage
2. `content_block_start` - Start text block
3. `content_block_delta` (x2) - Stream text chunks
4. `content_block_stop` - End text block
5. `message_delta` - Finalize with stop_reason
6. `message_stop` - Complete message

**Output:**
```
event: message_start
data: {"type":"message_start","message":{...}}

event: content_block_delta
data: {"type":"content_block_delta","delta":{"type":"text_delta","text":"Mock streaming "}}

...
```

### `tool_use_sse()` (Lines 831-914)
Builds SSE stream for tool use responses with chunked JSON.

### `final_text_sse()` (Lines 916-972)
Builds SSE stream for final text responses.

### `final_text_sse_with_usage()` (Lines 974-1040)
Like `final_text_sse` but with custom token counts.

### `append_sse()` (Lines 1043-1048)
```rust
fn append_sse(buffer: &mut String, event: &str, payload: Value) {
    use std::fmt::Write as _;
    writeln!(buffer, "event: {event}").expect("event write should succeed");
    writeln!(buffer, "data: {payload}").expect("payload write should succeed");
    buffer.push('\n');
}
```
Appends a single SSE event to the buffer.

### `usage_json()` (Lines 1050-1057)
```rust
fn usage_json(input_tokens: u32, output_tokens: u32) -> Value {
    json!({
        "input_tokens": input_tokens,
        "cache_creation_input_tokens": 0,
        "cache_read_input_tokens": 0,
        "output_tokens": output_tokens
    })
}
```

### `unique_message_id()` (Lines 1059-1065)
```rust
fn unique_message_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();
    format!("msg_{nanos}")
}
```
Generates unique message IDs from timestamp.

---

## Extraction Helpers (Lines 1067-1123)

These functions parse tool result JSON to extract specific fields.

### `extract_read_content()` (Lines 1067-1078)
```rust
fn extract_read_content(tool_output: &str) -> String {
    serde_json::from_str::<Value>(tool_output)
        .ok()
        .and_then(|value| {
            value
                .get("file")
                .and_then(|file| file.get("content"))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| tool_output.trim().to_string())
}
```
Extracts `file.content` from read_file results.

### `extract_num_matches()` (Lines 1080-1086)
Extracts `numMatches` from grep_search results.

### `extract_file_path()` (Lines 1088-1098)
Extracts `filePath` from write_file results.

### `extract_bash_stdout()` (Lines 1100-1110)
Extracts `stdout` from bash results.

### `extract_plugin_message()` (Lines 1112-1123)
Extracts `input.message` from plugin tool results.

---

## Main Binary (main.rs)

```rust
use std::env;

use mock_anthropic_service::MockAnthropicService;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut bind_addr = String::from("127.0.0.1:0");
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--bind" => {
                bind_addr = args
                    .next()
                    .ok_or_else(|| "missing value for --bind".to_string())?;
            }
            flag if flag.starts_with("--bind=") => {
                bind_addr = flag[7..].to_string();
            }
            "--help" | "-h" => {
                println!("Usage: mock-anthropic-service [--bind HOST:PORT]");
                return Ok(());
            }
            other => {
                return Err(format!("unsupported argument: {other}").into());
            }
        }
    }

    let server = MockAnthropicService::spawn_on(&bind_addr).await?;
    println!("MOCK_ANTHROPIC_BASE_URL={}", server.base_url());
    tokio::signal::ctrl_c().await?;
    drop(server);
    Ok(())
}
```

### Line-by-line:

- Line 5: Multi-threaded Tokio runtime
- Line 7: Default bind address (ephemeral port)
- Line 8-27: Argument parsing
  - Line 11-14: `--bind HOST:PORT` (two-arg form)
  - Line 16-18: `--bind=HOST:PORT` (single-arg form)
  - Line 19-22: `--help` / `-h`
  - Line 23-26: Unknown argument error
- Line 29: Spawn the mock server
- Line 30: Print the base URL for clients
- Line 31: Wait for Ctrl+C
- Line 32: Drop server (triggers graceful shutdown)

### Usage:
```bash
# Start on random port
mock-anthropic-service

# Start on specific port
mock-anthropic-service --bind 127.0.0.1:9999

# Capture URL for tests
export ANTHROPIC_BASE_URL=$(mock-anthropic-service & echo $!)
```

---

## Integration Points

### Upstream Dependencies
| Crate | Usage |
|-------|-------|
| `api` | MessageRequest, MessageResponse, content block types |

### Downstream Dependents
| Crate | How it uses mock-anthropic-service |
|-------|-----------------------------------|
| `claw-code` (workspace) | Dev dependency for integration tests |
| `compat-harness` | Parity test harness backend |

### Test Integration Pattern

```rust
#[tokio::test]
async fn test_parity_scenario() {
    // 1. Spawn mock server
    let mock = MockAnthropicService::spawn().await.unwrap();
    
    // 2. Configure client to use mock URL
    env::set_var("ANTHROPIC_BASE_URL", mock.base_url());
    
    // 3. Run claw-code with PARITY_SCENARIO: prompt
    // ...
    
    // 4. Assert captured requests
    let requests = mock.captured_requests().await;
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].scenario, "read_file_roundtrip");
}
```

---

## Summary

The mock-anthropic-service crate is a **complete HTTP server implementation** that:

| Component | Lines | Purpose |
|-----------|-------|---------|
| Service Lifecycle | 62 | Spawn/shutdown server |
| Scenario Enum | 51 | 12 test scenarios |
| HTTP Parser | 97 | Raw HTTP parsing |
| Scenario Detection | 65 | Parse prompts for scenarios |
| Response Builder | 329 | Scenario-specific responses |
| SSE Builders | 285 | Stream event construction |
| Helpers | 134 | Extraction, formatting |

**Key design patterns:**

1. **Scenario-based testing** - Prompts embed scenario identifier
2. **Two-turn flows** - Tool use → Tool result → Final text
3. **Request capture** - All requests stored for assertions
4. **Deterministic responses** - Same scenario = same response
5. **Graceful shutdown** - Drop-based cleanup

**Coverage:**
- All 12 scenarios cover distinct claw-code capabilities
- Both streaming and non-streaming modes
- Tool use, tool results, and final text responses
- Permission flows (allowed/denied)
- Multi-tool parallel execution
