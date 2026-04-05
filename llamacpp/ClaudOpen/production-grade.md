# Production-Grade Guide: Building a Resilient AI Agent System

**Purpose:** Guide for inexperienced engineers on building production-ready AI agent systems like ClaudOpen.

---

## Table of Contents

1. [Introduction](#introduction)
2. [System Architecture](#system-architecture)
3. [Building Blocks](#building-blocks)
4. [Data Flow](#data-flow)
5. [Animation and Rendering](#animation-and-rendering)
6. [Storage System Design](#storage-system-design)
7. [Error Handling](#error-handling)
8. [Testing Strategy](#testing-strategy)
9. [Deployment](#deployment)
10. [Monitoring and Observability](#monitoring-and-observability)
11. [Security Considerations](#security-considerations)
12. [Scaling](#scaling)

---

## Introduction

Building a production-grade AI agent system requires understanding multiple domains:

- **Network programming** (HTTP, SSE, WebSockets)
- **Terminal UI** (ANSI escapes, rendering)
- **Data persistence** (JSON, file I/O)
- **Process management** (spawning, IPC)
- **Error handling** (recovery, retry)
- **Security** (permissions, sandboxing)

This guide breaks down each component for engineers new to systems programming.

---

## System Architecture

### High-Level Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        User Interface                           │
│                    (Terminal/CLI/REPL)                          │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Application Layer                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │   Command   │  │   Session   │  │      Tool Router        │  │
│  │   Handler   │  │   Manager   │  │   (bash, file, web)     │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                       Runtime Layer                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │ Permission  │  │    Hook     │  │      MCP Manager        │  │
│  │   System    │  │   Runner    │  │   (stdio servers)       │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Integration Layer                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │  Anthropic  │  │   External  │  │      File System        │  │
│  │    API      │  │   Services  │  │      (local/remote)     │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Layer | Component | Responsibility |
|-------|-----------|----------------|
| UI | CLI/REPL | User input, output rendering |
| App | Command Handler | Parse and route commands |
| App | Session Manager | Load/save conversation state |
| App | Tool Router | Dispatch tool calls |
| Runtime | Permission System | Authorize actions |
| Runtime | Hook Runner | Execute pre/post hooks |
| Runtime | MCP Manager | Manage external servers |
| Integration | Anthropic API | LLM communication |
| Integration | External Services | Web search, fetch |
| Integration | File System | Read/write operations |

---

## Building Blocks

### 1. HTTP Client

**What it does:** Communicate with the Anthropic API

**Key concepts:**
- HTTP methods (POST for requests)
- Headers (authentication, content-type)
- Request/response bodies (JSON)
- Connection pooling

**Implementation:**

```rust
use reqwest::blocking::Client;
use serde::{Serialize, Deserialize};

#[derive(Serialize)]
struct MessageRequest {
    model: String,
    messages: Vec<Message>,
    max_tokens: u32,
    stream: bool,
}

#[derive(Deserialize)]
struct MessageResponse {
    id: String,
    content: Vec<ContentBlock>,
    usage: Usage,
}

pub struct AnthropicClient {
    client: Client,
    api_key: String,
    base_url: String,
}

impl AnthropicClient {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: "https://api.anthropic.com".to_string(),
        }
    }

    pub fn send(&self, request: MessageRequest) -> Result<MessageResponse, Error> {
        let url = format!("{}/v1/messages", self.base_url);

        let response = self.client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&request)
            .send()?;

        Ok(response.json()?)
    }
}
```

### 2. SSE Parser

**What it does:** Parse streaming server responses

**Key concepts:**
- Server-Sent Events format
- Event types and data parsing
- Buffer management

**Implementation:**

```rust
pub struct SseParser {
    buffer: String,
}

pub struct SseEvent {
    pub event_type: String,
    pub data: String,
}

impl SseParser {
    pub fn push(&mut self, chunk: &str) -> Vec<SseEvent> {
        self.buffer.push_str(chunk);

        // Split by double newline (event delimiter)
        let events: Vec<&str> = self.buffer.split("\n\n").collect();

        // Keep incomplete chunk in buffer
        self.buffer = events.last().unwrap_or(&"").to_string();

        events[..events.len()-1]
            .iter()
            .filter_map(|e| self.parse_event(e))
            .collect()
    }

    fn parse_event(&self, chunk: &str) -> Option<SseEvent> {
        let mut event_type = String::new();
        let mut data = String::new();

        for line in chunk.lines() {
            if let Some(rest) = line.strip_prefix("event: ") {
                event_type = rest.to_string();
            } else if let Some(rest) = line.strip_prefix("data: ") {
                data.push_str(rest);
            }
        }

        if data.is_empty() {
            return None;
        }

        Some(SseEvent { event_type, data })
    }
}
```

### 3. JSON-RPC Client (for MCP)

**What it does:** Communicate with MCP servers via stdio

**Key concepts:**
- JSON-RPC 2.0 protocol
- Request/response IDs
- Notifications (no response)

**Implementation:**

```rust
#[derive(Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,  // Always "2.0"
    id: u64,
    method: String,
    params: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: u64,
    result: Option<serde_json::Value>,
    error: Option<JsonRpcError>,
}

pub struct McpClient {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
}

impl McpClient {
    pub async fn send_request(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, Error> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: self.next_id,
            method: method.to_string(),
            params: Some(params),
        };
        self.next_id += 1;

        // Write request
        let request_json = serde_json::to_string(&request)?;
        writeln!(self.stdin, "{}", request_json)?;

        // Read response
        let mut response_line = String::new();
        self.stdout.read_line(&mut response_line)?;
        let response: JsonRpcResponse = serde_json::from_str(&response_line)?;

        if let Some(error) = response.error {
            return Err(Error::Rpc(error));
        }

        Ok(response.result.unwrap())
    }
}
```

---

## Data Flow

### Request Flow

```
1. User types: "Fix the bug in main.rs"
                │
                ▼
2. Input captured by rustyline
                │
                ▼
3. Added to session as User message
                │
                ▼
4. Build API request (system prompt + messages)
                │
                ▼
5. Send to Anthropic API
                │
                ▼
6. Receive SSE stream
                │
                ▼
7. Parse events, render to terminal
                │
                ▼
8. Extract tool calls from response
                │
                ▼
9. Check permissions for each tool
                │
                ▼
10. Execute tools (if allowed)
                │
                ▼
11. Add tool results to session
                │
                ▼
12. Loop back to step 4 (if more tool calls)
                │
                ▼
13. Save session to disk
```

### Session Persistence

```rust
#[derive(Serialize, Deserialize)]
pub struct Session {
    version: u32,
    messages: Vec<ConversationMessage>,
}

impl Session {
    pub fn save(&self, path: &Path) -> io::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn load(path: &Path) -> io::Result<Self> {
        let contents = fs::read_to_string(path)?;
        let session: Session = serde_json::from_str(&contents)?;
        Ok(session)
    }
}
```

---

## Animation and Rendering

### Spinner Animation

**What it does:** Show progress during API calls

**Concepts:**
- ANSI escape codes
- Cursor positioning
- Frame timing

**Implementation:**

```rust
use std::io::{Write, stdout};
use std::time::Duration;
use std::thread;

const FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub struct Spinner {
    frame: usize,
}

impl Spinner {
    pub fn new() -> Self {
        Self { frame: 0 }
    }

    pub fn tick(&mut self, label: &str) {
        let frame = FRAMES[self.frame % FRAMES.len()];
        self.frame += 1;

        // Move to start of line, clear, print frame
        print!("\r{} {}", frame, label);
        stdout().flush().unwrap();

        thread::sleep(Duration::from_millis(80));
    }

    pub fn finish(&mut self, label: &str) {
        print!("\r✔ {}\n", label);
        stdout().flush().unwrap();
    }
}

// Usage
let mut spinner = Spinner::new();
for _ in 0..50 {
    spinner.tick("Thinking...");
}
spinner.finish("Done!");
```

### Markdown Rendering

**What it does:** Convert Markdown to ANSI-formatted text

**Concepts:**
- Markdown parsing (pulldown-cmark)
- Syntax highlighting (syntect)
- ANSI colors

**Implementation:**

```rust
use pulldown_cmark::{Parser, Event, Tag};
use crossterm::style::{Color, Stylize};

pub struct TerminalRenderer;

impl TerminalRenderer {
    pub fn render(markdown: &str) -> String {
        let mut output = String::new();
        let parser = Parser::new(markdown);

        for event in parser {
            match event {
                Event::Start(Tag::Heading(1, ..)) => {
                    output.push_str("\x1b[36m\x1b[1m");  // Cyan, bold
                }
                Event::End(Tag::Heading(1, ..)) => {
                    output.push_str("\x1b[0m\n\n");  // Reset
                }
                Event::Start(Tag::Strong) => {
                    output.push_str("\x1b[1m");  // Bold
                }
                Event::End(Tag::Strong) => {
                    output.push_str("\x1b[0m");  // Reset
                }
                Event::Text(text) => {
                    output.push_str(&text);
                }
                _ => {}
            }
        }

        output
    }
}
```

---

## Storage System Design

### File Structure

```
.claude/
├── settings.json          # User configuration
├── settings.local.json    # Local overrides (gitignored)
└── sessions/
    ├── session-001.json   # Conversation history
    ├── session-002.json
    └── ...
```

### Configuration Hierarchy

```
1. ~/.claude/settings.json     (User global)
2. ./.claude/settings.json     (Project)
3. ./.claude/settings.local.json (Local overrides)
```

### Merging Configuration

```rust
use std::collections::BTreeMap;
use serde_json::Value;

fn merge_configs(base: &mut Value, override_val: &Value) {
    if let (Some(base_obj), Some(override_obj)) =
        (base.as_object_mut(), override_val.as_object())
    {
        for (key, value) in override_obj {
            if let Some(base_value) = base_obj.get_mut(key) {
                if base_value.is_object() && value.is_object() {
                    merge_configs(base_value, value);
                } else {
                    *base_value = value.clone();
                }
            } else {
                base_obj.insert(key.clone(), value.clone());
            }
        }
    }
}
```

### Session Compaction

When sessions grow too large, compact old messages:

```rust
pub fn compact_session(session: &Session, preserve_count: usize) -> Session {
    if session.messages.len() <= preserve_count {
        return session.clone();
    }

    // Generate summary of old messages
    let summary = generate_summary(&session.messages[..session.messages.len() - preserve_count]);

    // Build new session with summary
    let mut compacted = Session::new();
    compacted.messages.push(Message::system(summary));

    // Add preserved messages
    for msg in session.messages.iter().skip(session.messages.len() - preserve_count) {
        compacted.messages.push(msg.clone());
    }

    compacted
}
```

---

## Error Handling

### Error Categories

| Category | Examples | Recovery |
|----------|----------|----------|
| Transient | Network timeout, rate limit | Retry with backoff |
| Permanent | Invalid API key, bad JSON | Fail with message |
| User Error | File not found, permission denied | Prompt user |
| System Error | Disk full, process killed | Graceful degradation |

### Retry Logic

```rust
use std::time::Duration;
use std::thread;

pub fn retry_with_backoff<T, E, F>(
    mut operation: F,
    max_retries: u32,
) -> Result<T, E>
where
    F: FnMut() -> Result<T, E>,
    E: std::fmt::Debug,
{
    let mut retry = 0;
    loop {
        match operation() {
            Ok(result) => return Ok(result),
            Err(e) => {
                retry += 1;
                if retry >= max_retries {
                    return Err(e);
                }

                // Exponential backoff
                let delay = Duration::from_millis(100 * 2u64.pow(retry - 1));
                thread::sleep(delay);
            }
        }
    }
}

// Usage
let result = retry_with_backoff(|| client.send(request), 3)?;
```

### Graceful Degradation

```rust
pub fn load_config() -> Config {
    // Try user config
    if let Ok(config) = Config::load_user() {
        return config;
    }

    // Fall back to defaults
    Config::default()
}

pub fn save_session(session: &Session) {
    if let Err(e) = session.save() {
        eprintln!("Warning: Failed to save session: {}", e);
        // Continue anyway - don't crash
    }
}
```

---

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_configs() {
        let mut base = json!({"a": 1, "b": 2});
        let override_val = json!({"b": 3, "c": 4});

        merge_configs(&mut base, &override_val);

        assert_eq!(base, json!({"a": 1, "b": 3, "c": 4}));
    }

    #[test]
    fn test_sse_parser() {
        let mut parser = SseParser::new();
        let events = parser.push("event: test\ndata: hello\n\n");

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "test");
        assert_eq!(events[0].data, "hello");
    }
}
```

### Integration Tests

```rust
// tests/api_integration.rs
#[test]
#[ignore]  // Skip by default (requires API key)
fn test_real_api_call() {
    let api_key = std::env::var("ANTHROPIC_API_KEY").unwrap();
    let client = AnthropicClient::new(api_key);

    let request = MessageRequest {
        model: "claude-sonnet-4-6".to_string(),
        messages: vec![Message::user("Hello")],
        max_tokens: 100,
        stream: false,
    };

    let response = client.send(request).unwrap();
    assert!(!response.content.is_empty());
}
```

### Mock Objects

```rust
// Mock API client for testing
struct MockApiClient {
    responses: Vec<Vec<AssistantEvent>>,
    call_count: usize,
}

impl ApiClient for MockApiClient {
    fn stream(&mut self, _request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
        let response = self.responses[self.call_count].clone();
        self.call_count += 1;
        Ok(response)
    }
}

// Usage in tests
let mock_client = MockApiClient {
    responses: vec![
        vec![AssistantEvent::TextDelta("Hello".to_string())],
        vec![AssistantEvent::ToolUse { ... }],
    ],
    call_count: 0,
};
```

---

## Deployment

### Building for Release

```bash
# Build optimized binary
cargo build --release

# Strip debug symbols
strip target/release/claw

# Binary is now at:
ls -lh target/release/claw
```

### Distribution

```bash
# Create release archive
tar -czvf claw-linux-x64.tar.gz -C target/release claw

# Upload to GitHub releases
gh release create v1.0.0 claw-linux-x64.tar.gz
```

### Installation Script

```bash
#!/bin/bash
# install.sh

VERSION="1.0.0"
ARCH="x86_64"
OS="linux"

DOWNLOAD_URL="https://github.com/instructkr/claw-code/releases/download/v${VERSION}/claw-${OS}-${ARCH}.tar.gz"

curl -L "$DOWNLOAD_URL" | tar xz -C /usr/local/bin

echo "Claw installed successfully!"
```

---

## Monitoring and Observability

### Logging

```rust
use tracing::{info, warn, error, instrument};

#[instrument(skip(client, request))]
pub fn stream_with_logging(
    client: &mut AnthropicClient,
    request: MessageRequest,
) -> Result<Vec<AssistantEvent>, ApiError> {
    let start = Instant::now();

    info!("Starting API request");

    match client.stream(request) {
        Ok(events) => {
            let duration = start.elapsed();
            info!(
                duration_ms = duration.as_millis(),
                events_count = events.len(),
                "API request completed"
            );
            Ok(events)
        }
        Err(e) => {
            error!(error = %e, "API request failed");
            Err(e)
        }
    }
}
```

### Metrics

```rust
use std::sync::atomic::{AtomicU64, Ordering};

static API_CALLS: AtomicU64 = AtomicU64::new(0);
static API_ERRORS: AtomicU64 = AtomicU64::new(0);
static TOTAL_TOKENS: AtomicU64 = AtomicU64::new(0);

pub fn record_api_call() {
    API_CALLS.fetch_add(1, Ordering::Relaxed);
}

pub fn record_api_error() {
    API_ERRORS.fetch_add(1, Ordering::Relaxed);
}

pub fn record_tokens(tokens: u64) {
    TOTAL_TOKENS.fetch_add(tokens, Ordering::Relaxed);
}

pub fn get_metrics() -> Metrics {
    Metrics {
        api_calls: API_CALLS.load(Ordering::Relaxed),
        api_errors: API_ERRORS.load(Ordering::Relaxed),
        total_tokens: TOTAL_TOKENS.load(Ordering::Relaxed),
    }
}
```

---

## Security Considerations

### Permission System

```rust
pub enum PermissionMode {
    ReadOnly,         // Can read files, search
    WorkspaceWrite,   // Can edit files
    DangerFullAccess, // Can run any command
}

pub fn authorize_tool(
    tool_name: &str,
    current_mode: PermissionMode,
    required_mode: PermissionMode,
) -> PermissionOutcome {
    if current_mode >= required_mode {
        return PermissionOutcome::Allow;
    }

    // Prompt user for approval
    prompt_user_for_approval(tool_name, required_mode)
}
```

### Sandboxing

```rust
// Restrict bash to workspace directory
pub fn execute_bash_sandboxed(command: &str, workspace: &Path) -> Result<String, Error> {
    let output = Command::new("bash")
        .arg("-c")
        .arg(command)
        .current_dir(workspace)
        .env("PATH", workspace.join("bin"))  // Restrict PATH
        .output()?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(Error::CommandFailed(String::from_utf8_lossy(&output.stderr).to_string()))
    }
}
```

### Secret Handling

```rust
// Never log API keys
pub struct RedactingFormatter;

impl tracing::field::Visit for RedactingFormatter {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "api_key" || field.name() == "token" {
            self.buffer.push_str("[REDACTED]");
        } else {
            self.buffer.push_str(value);
        }
    }
}
```

---

## Scaling

### Connection Pooling

```rust
use reqwest::Client;
use std::time::Duration;

pub fn create_pooled_client() -> Client {
    Client::builder()
        .pool_max_idle_per_host(10)
        .pool_idle_timeout(Duration::from_secs(90))
        .timeout(Duration::from_secs(600))
        .build()
        .unwrap()
}
```

### Rate Limiting

```rust
use governor::{Quota, RateLimiter};
use std::num::NonZeroU32;

pub struct RateLimitedClient {
    client: AnthropicClient,
    limiter: RateLimiter,
}

impl RateLimitedClient {
    pub fn new() -> Self {
        // 60 requests per minute
        let quota = Quota::per_minute(NonZeroU32::new(60).unwrap());
        Self {
            client: AnthropicClient::new(),
            limiter: RateLimiter::direct(quota),
        }
    }

    pub async fn stream(&self, request: MessageRequest) -> Result<Vec<AssistantEvent>, Error> {
        self.limiter.until_ready().await;
        self.client.stream(request).await
    }
}
```

---

## Checklist for Production Readiness

### Code Quality
- [ ] All functions have error handling
- [ ] No unwrap() in production code
- [ ] Comprehensive test coverage
- [ ] Documentation for public APIs

### Reliability
- [ ] Retry logic for transient failures
- [ ] Graceful degradation on errors
- [ ] Session persistence with recovery
- [ ] Rate limiting for external APIs

### Security
- [ ] Permission system enforced
- [ ] Secrets never logged
- [ ] Input validation on all user input
- [ ] Sandbox for external commands

### Performance
- [ ] Connection pooling enabled
- [ ] Caching for repeated requests
- [ ] Memory-efficient streaming
- [ ] Release builds optimized

### Observability
- [ ] Structured logging
- [ ] Metrics collection
- [ ] Error reporting
- [ ] Performance profiling

---

## References

- [The Rust Programming Language](https://doc.rust-lang.org/book/)
- [Designing Data-Intensive Applications](https://dataintensive.net/)
- [Building Microservices](https://www.oreilly.com/library/view/building-microservices/9781491950340/)

---

*Generated: 2026-04-02*
