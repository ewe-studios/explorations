# Rust Revision: Idiomatic Rust Implementation Patterns

**Source:** ClaudOpen Rust Implementation
**Repository:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claw-code/rust`

---

## Table of Contents

1. [Overview](#overview)
2. [Crate Structure](#crate-structure)
3. [Type System Design](#type-system-design)
4. [Error Handling Strategy](#error-handling-strategy)
5. [Concurrency and Async](#concurrency-and-async)
6. [Memory Management](#memory-management)
7. [Trait Abstractions](#trait-abstractions)
8. [Testing Strategy](#testing-strategy)
9. [Performance Optimizations](#performance-optimizations)
10. [Safety Guarantees](#safety-guarantees)

---

## Overview

The ClaudOpen Rust implementation demonstrates production-grade Rust patterns:

- **Workspace organization** with 6 focused crates
- **Trait-based abstractions** for testability
- **Comprehensive error types** with context
- **Zero unsafe code** (forbidden in lints)
- **Extensive test coverage** with scripted clients

### Key Metrics

| Metric | Value |
|--------|-------|
| Total Lines | ~20,000 |
| Crates | 6 |
| Unsafe Code | 0 (forbidden) |
| Test Coverage | ~40% |
| Binary Size | 42MB (release) |

---

## Crate Structure

### Workspace Layout

```toml
# Cargo.toml
[workspace]
members = ["crates/*"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT"
publish = false

[workspace.lints.rust]
unsafe_code = "forbid"

[workspace.lints.clippy]
all = { level = "warn", priority = -1 }
pedantic = { level = "warn", priority = -1 }
module_name_repetitions = "allow"
missing_panics_doc = "allow"
missing_errors_doc = "allow"
```

### Crate Responsibilities

```
crates/
├── api/                # HTTP client, SSE parsing
│   ├── lib.rs          # Public API
│   ├── client.rs       # AnthropicClient
│   ├── types.rs        # Request/response types
│   ├── sse.rs          # SSE parser
│   └── error.rs        # ApiError
│
├── commands/           # Slash command registry
│   └── lib.rs          # Command specs
│
├── compat-harness/     # TS manifest extraction
│   └── lib.rs          # Manifest extraction
│
├── runtime/            # Core agentic loop
│   ├── lib.rs          # Public exports
│   ├── conversation.rs # ConversationRuntime
│   ├── session.rs      # Session persistence
│   ├── config.rs       # Config loading
│   ├── permissions.rs  # Permission system
│   ├── prompt.rs       # Prompt builder
│   ├── mcp_stdio.rs    # MCP stdio transport
│   └── ...
│
├── rusty-claude-cli/   # Main CLI binary
│   ├── main.rs         # Entry point
│   ├── render.rs       # Terminal rendering
│   └── input.rs        # Line editor
│
└── tools/              # Tool implementations
    └── lib.rs          # Tool specs + execution
```

---

## Type System Design

### Newtype Patterns

```rust
// Strong typing for IDs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionId(String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolUseId(String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageId(String);

// Usage prevents mixing up token types
#[derive(Debug, Clone, Default)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_creation_input_tokens: u32,
    pub cache_read_input_tokens: u32,
}

// Permission modes are distinct types, not strings
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PermissionMode {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
    Prompt,
    Allow,
}
```

### Tagged Enums for Protocol Types

```rust
// Content blocks use tag for serde
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text { text: String },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool_use_id: String,
        content: Vec<ToolResultContentBlock>,
    },
}

// Tool choice is a tagged enum
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolChoice {
    Auto,
    Any,
    Tool { name: String },
}
```

### Builder Pattern

```rust
#[derive(Debug, Clone, Default)]
pub struct SystemPromptBuilder {
    output_style_name: Option<String>,
    output_style_prompt: Option<String>,
    os_name: Option<String>,
    os_version: Option<String>,
    append_sections: Vec<String>,
    project_context: Option<ProjectContext>,
    config: Option<RuntimeConfig>,
}

impl SystemPromptBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_output_style(
        mut self,
        name: impl Into<String>,
        prompt: impl Into<String>
    ) -> Self {
        self.output_style_name = Some(name.into());
        self.output_style_prompt = Some(prompt.into());
        self
    }

    pub fn with_os(
        mut self,
        os_name: impl Into<String>,
        os_version: impl Into<String>
    ) -> Self {
        self.os_name = Some(os_name.into());
        self.os_version = Some(os_version.into());
        self
    }

    pub fn with_project_context(mut self, ctx: ProjectContext) -> Self {
        self.project_context = Some(ctx);
        self
    }

    pub fn with_runtime_config(mut self, config: RuntimeConfig) -> Self {
        self.config = Some(config);
        self
    }

    pub fn append_section(mut self, section: impl Into<String>) -> Self {
        self.append_sections.push(section.into());
        self
    }

    pub fn build(self) -> Vec<String> {
        // Build sections...
    }

    pub fn render(self) -> String {
        self.build().join("\n\n")
    }
}
```

### Generic Programming

```rust
// ConversationRuntime is generic over client and executor
pub struct ConversationRuntime<C, T>
where
    C: ApiClient,
    T: ToolExecutor,
{
    session: Session,
    api_client: C,
    tool_executor: T,
    permission_policy: PermissionPolicy,
    system_prompt: Vec<String>,
    max_iterations: usize,
    usage_tracker: UsageTracker,
    hook_runner: HookRunner,
}

impl<C, T> ConversationRuntime<C, T>
where
    C: ApiClient,
    T: ToolExecutor,
{
    pub fn new(
        session: Session,
        api_client: C,
        tool_executor: T,
        permission_policy: PermissionPolicy,
        system_prompt: Vec<String>,
    ) -> Self {
        // ...
    }

    pub fn run_turn(
        &mut self,
        user_input: impl Into<String>,
        prompter: Option<&mut dyn PermissionPrompter>,
    ) -> Result<TurnSummary, RuntimeError> {
        // Generic over C and T
    }
}
```

---

## Error Handling Strategy

### Custom Error Types

```rust
// api/src/error.rs
#[derive(Debug)]
pub enum ApiError {
    Http(reqwest::Error),
    Json(serde_json::Error),
    Io(io::Error),
    Authentication { message: String },
    RateLimit { retry_after: Option<Duration> },
    ServerError { status: u16, message: String },
    UnknownEvent(String),
    NoCredentials,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::Http(e) => write!(f, "HTTP error: {}", e),
            ApiError::Json(e) => write!(f, "JSON error: {}", e),
            ApiError::Authentication { message } => {
                write!(f, "Authentication failed: {}", message)
            }
            ApiError::RateLimit { retry_after } => {
                write!(f, "Rate limit exceeded")?;
                if let Some(duration) = retry_after {
                    write!(f, ", retry after {}s", duration.as_secs())?;
                }
                Ok(())
            }
            ApiError::ServerError { status, message } => {
                write!(f, "Server error ({}): {}", status, message)
            }
            _ => write!(f, "{:?}", self),
        }
    }
}

impl std::error::Error for ApiError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ApiError::Http(e) => Some(e),
            ApiError::Json(e) => Some(e),
            ApiError::Io(e) => Some(e),
            _ => None,
        }
    }
}

// From implementations for ? operator
impl From<reqwest::Error> for ApiError {
    fn from(error: reqwest::Error) -> Self {
        ApiError::Http(error)
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(error: serde_json::Error) -> Self {
        ApiError::Json(error)
    }
}

impl From<io::Error> for ApiError {
    fn from(error: io::Error) -> Self {
        ApiError::Io(error)
    }
}
```

### Result Aliases

```rust
// Type aliases for common Result types
pub type ApiResult<T> = Result<T, ApiError>;
pub type RuntimeError = String;
pub type ToolError = String;
pub type SessionResult<T> = Result<T, SessionError>;
pub type ConfigResult<T> = Result<T, ConfigError>;
```

### Context-Rich Errors

```rust
#[derive(Debug)]
pub enum SessionError {
    Io(std::io::Error),
    Json(JsonError),
    Format(String),  // Human-readable context
}

impl Display for SessionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::Json(error) => write!(f, "{error}"),
            Self::Format(error) => write!(f, "{error}"),
        }
    }
}

// Usage with context
pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, SessionError> {
    let contents = fs::read_to_string(&path).map_err(|e| {
        SessionError::Io(io::Error::new(
            e.kind(),
            format!("Failed to read session file at {}: {}", path.as_ref().display(), e),
        ))
    })?;
    // ...
}
```

---

## Concurrency and Async

### Blocking vs Async Trade-offs

The current implementation uses **blocking I/O** for simplicity:

```rust
use reqwest::blocking::{Client, Response};

pub fn stream(&mut self, request: MessageRequest) -> Result<Vec<AssistantEvent>, ApiError> {
    let response = self.http_client.post(&url).json(&request).send()?;
    parse_sse_stream(response)
}
```

### Recommended Async Pattern

```rust
use reqwest::Client;
use tokio::io::AsyncBufReadExt;

pub async fn stream(
    &self,
    request: MessageRequest,
) -> Result<Vec<AssistantEvent>, ApiError> {
    let url = format!("{}/v1/messages", self.base_url);

    let response = self.http_client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("anthropic-version", "2023-06-01")
        .json(&request)
        .send()
        .await?;

    let mut stream = response.bytes_stream();
    let mut parser = SseParser::new();
    let mut events = Vec::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        let text = String::from_utf8_lossy(&chunk);
        let frames = parser.push(&text);

        for frame in frames {
            if let Some(event) = parse_stream_event(&frame)? {
                events.push(event);
            }
        }
    }

    Ok(events)
}
```

### Tokio Runtime

```rust
// For async, use tokio runtime
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = AnthropicClient::new()?;
    let events = client.stream(request).await?;
    // ...
    Ok(())
}
```

### Thread Safety

```rust
// Use Arc for shared state across threads
use std::sync::Arc;

pub struct SharedState {
    config: Arc<RuntimeConfig>,
    session_store: Arc<Mutex<SessionStore>>,
}

impl Clone for SharedState {
    fn clone(&self) -> Self {
        Self {
            config: Arc::clone(&self.config),
            session_store: Arc::clone(&self.session_store),
        }
    }
}
```

---

## Memory Management

### Zero-Copy Parsing

```rust
// Use Cow for zero-copy when possible
use std::borrow::Cow;

pub enum Event<'a> {
    Text(Cow<'a, str>),
    Code(Cow<'a, str>),
    // ...
}

// SSE parsing with borrowed data
pub fn parse_frame<'a>(data: &'a str) -> Option<SseFrame<'a>> {
    // Parse without allocating when possible
}
```

### Buffer Reuse

```rust
// Pre-allocate buffers
let mut output = String::with_capacity(markdown.len() * 2);
let mut buffer = Vec::with_capacity(4096);

// Reuse buffers across iterations
buffer.clear();
buffer.extend_from_slice(&new_data);
```

### Lazy Evaluation

```rust
// Use iterators instead of collecting
let tool_names = session.messages
    .iter()
    .flat_map(|msg| &msg.blocks)
    .filter_map(|block| match block {
        ContentBlock::ToolUse { name, .. } => Some(name),
        _ => None,
    });

// Only collect when needed
let names: Vec<_> = tool_names.collect();
```

---

## Trait Abstractions

### Core Traits

```rust
// API client trait for testability
pub trait ApiClient {
    fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError>;
}

// Tool executor trait
pub trait ToolExecutor {
    fn execute(&mut self, tool_name: &str, input: &str) -> Result<String, ToolError>;
}

// Permission prompter trait
pub trait PermissionPrompter {
    fn decide(&mut self, request: &PermissionRequest) -> PermissionPromptDecision;
}
```

### Mock Implementations

```rust
// Scripted client for testing
struct ScriptedApiClient {
    call_count: usize,
}

impl ApiClient for ScriptedApiClient {
    fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
        self.call_count += 1;
        match self.call_count {
            1 => Ok(vec![
                AssistantEvent::TextDelta("Let me calculate.".to_string()),
                AssistantEvent::ToolUse {
                    id: "tool-1".to_string(),
                    name: "add".to_string(),
                    input: "2,2".to_string(),
                },
                AssistantEvent::MessageStop,
            ]),
            2 => Ok(vec![
                AssistantEvent::TextDelta("The answer is 4.".to_string()),
                AssistantEvent::MessageStop,
            ]),
            _ => Err(RuntimeError::new("unexpected extra API call")),
        }
    }
}

// Recording prompter for testing
struct RecordingPrompter {
    seen: Vec<PermissionRequest>,
    allow: bool,
}

impl PermissionPrompter for RecordingPrompter {
    fn decide(&mut self, request: &PermissionRequest) -> PermissionPromptDecision {
        self.seen.push(request.clone());
        if self.allow {
            PermissionPromptDecision::Allow
        } else {
            PermissionPromptDecision::Deny {
                reason: "not now".to_string(),
            }
        }
    }
}
```

### Static Tool Executor

```rust
// Builder pattern for test tool executors
#[derive(Default)]
pub struct StaticToolExecutor {
    handlers: BTreeMap<String, ToolHandler>,
}

type ToolHandler = Box<dyn FnMut(&str) -> Result<String, ToolError>>;

impl StaticToolExecutor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(
        mut self,
        tool_name: impl Into<String>,
        handler: impl FnMut(&str) -> Result<String, ToolError> + 'static,
    ) -> Self {
        self.handlers.insert(tool_name.into(), Box::new(handler));
        self
    }
}

impl ToolExecutor for StaticToolExecutor {
    fn execute(&mut self, tool_name: &str, input: &str) -> Result<String, ToolError> {
        self.handlers
            .get_mut(tool_name)
            .ok_or_else(|| ToolError::new(format!("unknown tool: {tool_name}")))?(input)
    }
}

// Usage in tests
let tool_executor = StaticToolExecutor::new()
    .register("add", |input| {
        let total = input
            .split(',')
            .map(|part| part.parse::<i32>().unwrap())
            .sum::<i32>();
        Ok(total.to_string())
    });
```

---

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_user_to_tool_to_result_loop_end_to_end() {
        let api_client = ScriptedApiClient { call_count: 0 };
        let tool_executor = StaticToolExecutor::new()
            .register("add", |input| {
                let total = input
                    .split(',')
                    .map(|part| part.parse::<i32>().expect("valid integer"))
                    .sum::<i32>();
                Ok(total.to_string())
            });
        let permission_policy = PermissionPolicy::new(PermissionMode::WorkspaceWrite);

        let mut runtime = ConversationRuntime::new(
            Session::new(),
            api_client,
            tool_executor,
            permission_policy,
            vec!["system".to_string()],
        );

        let summary = runtime
            .run_turn("what is 2 + 2?", Some(&mut PromptAllowOnce))
            .expect("conversation loop should succeed");

        assert_eq!(summary.iterations, 2);
        assert_eq!(summary.assistant_messages.len(), 2);
        assert_eq!(summary.tool_results.len(), 1);
        assert_eq!(runtime.session().messages.len(), 4);
    }

    #[test]
    fn records_denied_tool_results_when_prompt_rejects() {
        struct RejectPrompter;
        impl PermissionPrompter for RejectPrompter {
            fn decide(&mut self, _request: &PermissionRequest) -> PermissionPromptDecision {
                PermissionPromptDecision::Deny {
                    reason: "not now".to_string(),
                }
            }
        }

        // Test implementation...
    }
}
```

### Integration Tests

```rust
// api/tests/client_integration.rs
#[test]
#[ignore]  // Requires API key
fn test_real_api_streaming() {
    let mut client = AnthropicClient::new().unwrap();
    let request = MessageRequest {
        model: "claude-sonnet-4-6".to_string(),
        messages: vec![InputMessage {
            role: "user".to_string(),
            content: vec![InputContentBlock::Text {
                text: "Hello".to_string(),
            }],
        }],
        max_tokens: 100,
        stream: true,
        // ...
    };

    let events = client.stream(request).unwrap();
    assert!(!events.is_empty());
}
```

### Test Commands

```bash
# Run all tests
cargo test --workspace

# Run with output
cargo test --workspace -- --nocapture

# Run specific test
cargo test -p runtime runs_user_to_tool_to_result_loop_end_to_end

# Run with coverage
cargo llvm-cov --workspace --html

# Check documentation
cargo doc --workspace --no-deps
```

---

## Performance Optimizations

### Release Profile

```toml
# Cargo.toml
[profile.release]
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

### Lazy Static Initialization

```rust
use lazy_static::lazy_static;
use syntect::parsing::SyntaxSet;
use syntect::highlighting::ThemeSet;

lazy_static! {
    static ref SYNTAX_SET: SyntaxSet = SyntaxSet::load_defaults_newlines();
    static ref THEME_SET: ThemeSet = ThemeSet::load_defaults();
}
```

### Pre-computed Tables

```rust
// Pricing lookup table
lazy_static! {
    static ref PRICING_TABLE: HashMap<&'static str, ModelPricing> = {
        let mut table = HashMap::new();
        table.insert("opus", ModelPricing { /* ... */ });
        table.insert("sonnet", ModelPricing { /* ... */ });
        table.insert("haiku", ModelPricing { /* ... */ });
        table
    };
}
```

---

## Safety Guarantees

### Lint Configuration

```toml
[workspace.lints.rust]
unsafe_code = "forbid"

[workspace.lints.clippy]
all = { level = "warn", priority = -1 }
pedantic = { level = "warn", priority = -1 }
```

### No Unsafe Code

```rust
// The entire codebase has zero unsafe blocks
// This is enforced by the `forbid` lint
```

### Panic-Free Operations

```rust
// Use Result instead of panics
pub fn parse(value: &str) -> Result<Self, ParseError> {
    // Return errors instead of panicking
}

// Use unwrap_or_else for defaults
let value = config.get("key").unwrap_or_else(|| default_value());
```

---

## References

- [Rust Book](https://doc.rust-lang.org/book/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)

---

*Generated: 2026-04-02*
