# Agents Deep Dive: Agent Loop, Streaming, Prompt Assembly, Tool Execution

The agents crate (`moltis-agents`) manages the agent loop, streaming, prompt assembly, and tool execution. At 46 files and ~20,280 lines of Rust, it's the brain of Moltis.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Moltis/moltis/crates/agents/src/`

## Crate Structure (46 files)

### Core Runner

| Module | Purpose |
|--------|---------|
| `runner/mod.rs` | Agent runner entry point, re-exports |
| `runner/streaming.rs` | Streaming agent loop with real-time UI updates |
| `runner/non_streaming.rs` | Non-streaming agent loop |
| `runner/helpers.rs` | Agent loop limits, error types, event types |
| `runner/retry.rs` | Retry logic for rate limits and context windows |
| `runner/tool_result.rs` | Tool result extraction and sanitization |

### Model Layer

| Module | Purpose |
|--------|---------|
| `model/mod.rs` | Model types re-exports |
| `model/chat.rs` | Chat message types |
| `model/convert.rs` | Message conversion utilities |
| `model/stream.rs` | Stream event types |
| `model/types.rs` | LLM provider interface, tool types |

### Prompt System

| Module | Purpose |
|--------|---------|
| `prompt.rs` | Prompt module entry point |
| `prompt/builder.rs` | Prompt builder for assembling system + user messages |
| `prompt/formatting.rs` | Prompt formatting utilities |
| `prompt/types.rs` | Prompt type definitions |
| `prompt/tests.rs` | Prompt tests |

### Tool System

| Module | Purpose |
|--------|---------|
| `tool_registry.rs` | Tool registration and lookup |
| `tool_parsing.rs` | Parse tool calls from LLM response text |
| `tool_arg_validator.rs` | Validate tool arguments against schemas |
| `tool_loop_detector.rs` | Detect infinite tool call loops |
| `response_sanitizer.rs` | Sanitize LLM responses |
| `json_repair.rs` | Repair malformed JSON from LLM |

### Features

| Module | Purpose |
|--------|---------|
| `auth_profiles.rs` | Authentication profiles for agents |
| `docs.rs` | Documentation injection into prompts |
| `lazy_tools.rs` | Lazy tool loading |
| `memory_writer.rs` | Memory write operations |
| `multimodal.rs` | Multimodal (image) support |
| `provider_chain.rs` | Provider fallback chain |
| `skills.rs` | Skill definitions |
| `silent_turn.rs` | Silent (no-output) agent turns |
| `title.rs` | Conversation title generation |

## runner/mod.rs: Agent Runner Entry Point

**Source:** `crates/agents/src/runner/mod.rs` (53 lines)

### Re-exports

```rust
// crates/agents/src/runner/mod.rs
pub use helpers::{AgentLoopLimits, AgentRunError, AgentRunResult, OnEvent, RunnerEvent};
pub use non_streaming::{run_agent, run_agent_loop, run_agent_loop_with_context, run_agent_loop_with_context_and_limits};
pub use streaming::{run_agent_loop_streaming, run_agent_loop_streaming_with_limits};
pub use tool_result::{ExtractedImage, sanitize_tool_result, tool_result_to_content};
```

### SteerInbox

```rust
// crates/agents/src/runner/mod.rs:32
pub type SteerInbox = std::sync::Arc<tokio::sync::Mutex<Vec<String>>>;
```

Shared inbox for mid-flight steering text (populated by `/steer` command). The agent loop drains this between iterations and injects the text as a system notice so the LLM sees the guidance on its next call.

## runner/streaming.rs: Streaming Agent Loop

**Source:** `crates/agents/src/runner/streaming.rs` (~600 lines)

### run_agent_loop_streaming()

```rust
// crates/agents/src/runner/streaming.rs:57-83
pub async fn run_agent_loop_streaming(
    provider: Arc<dyn LlmProvider>,
    tools: &ToolRegistry,
    system_prompt: &str,
    user_content: &UserContent,
    on_event: Option<&OnEvent>,
    history: Option<Vec<ChatMessage>>,
    tool_context: Option<serde_json::Value>,
    hook_registry: Option<Arc<HookRegistry>>,
    sender_name: Option<String>,
    steer_inbox: Option<super::SteerInbox>,
) -> Result<AgentRunResult, AgentRunError>
```

The streaming agent loop. Unlike the non-streaming variant, this sends text deltas to the UI as they arrive, providing real-time UX. Tool calls are accumulated from the stream and executed after the stream completes, then the loop continues.

### The Streaming Loop

1. **Initialize** — Load config, set up limits, prepare history
2. **Before agent start hook** — Dispatch hook if registered
3. **Loop iteration** (up to `limits.max_iterations`):
   a. **Check steer inbox** — Drain any steering text and inject as system notice
   b. **Apply loop detector intervention** — Check for tool loops
   c. **Before LLM call hook** — Modify payload if needed
   d. **Call provider** — Stream LLM response
   e. **Process stream** — Accumulate text, tool calls, usage
   f. **Dispatch events** — Send text deltas, tool calls to UI
   g. **After LLM call hook** — Post-processing
   h. **Check for tool calls**:
      - If no tool calls and substantive answer → finish
      - If tool calls → execute each tool, accumulate results
   i. **Check limits** — Token budget, iteration count
4. **Finish** — Return `AgentRunResult` with accumulated text, usage, tool calls

### Stream Processing

```rust
// Process each stream event:
// - Text delta → emit to UI, accumulate in answer
// - Tool call → accumulate in tool_calls list
// - Usage → accumulate in usage counter
// - Raw event → push to capped event list (for debugging)
```

### Tool Loop Detection

```rust
let mut loop_detector = ToolLoopDetector::new();
// For each tool call:
let fingerprint = ToolCallFingerprint::from(&tool_call);
if loop_detector.is_loop(&fingerprint) {
    // Break the loop by returning early or injecting a nudge
}
```

### Retry Logic

```rust
// crates/agents/src/runner/retry.rs
// - Rate limit errors → exponential backoff, up to RATE_LIMIT_MAX_RETRIES
// - Context window errors → compact tool results, retry
// - Other errors → fail immediately
```

## runner/non_streaming.rs: Non-Streaming Agent Loop

Similar to streaming but processes the full response at once instead of streaming deltas.

## runner/helpers.rs: Helper Functions

**Source:** `crates/agents/src/runner/helpers.rs` (~500 lines)

### AgentLoopLimits

```rust
pub struct AgentLoopLimits {
    pub max_iterations: Option<usize>,      // Max loop iterations
    pub max_tokens: Option<usize>,          // Max token budget
    pub max_tool_calls: Option<usize>,      // Max tool calls per iteration
}
```

### AgentRunError / AgentRunResult

```rust
pub enum AgentRunError {
    ProviderError(String),      // LLM provider failure
    ToolExecutionError(String), // Tool execution failure
    ContextWindowExceeded,      // Context window overflow
    RateLimited,                // Rate limit hit
    Steered(String),            // User steered the conversation
}

pub struct AgentRunResult {
    pub text: String,                    // Accumulated answer text
    pub tool_calls: Vec<ToolCall>,       // All tool calls made
    pub tool_results: Vec<ToolResult>,   // All tool results
    pub usage: Usage,                    // Token usage
    pub history: Vec<ChatMessage>,       // Full conversation history
}
```

### Key Helper Functions

#### `has_named_tool_call()`

Checks if the LLM response contains any named tool calls (as opposed to text that looks like tool calls but isn't).

#### `find_empty_tool_name_call()`

Finds tool calls with empty names — triggers a retry with the error message.

#### `sanitize_tool_name()`

Sanitizes tool names to prevent injection attacks.

#### `apply_loop_detector_intervention()`

If a tool loop is detected, injects a nudge to break the loop or returns early.

#### `dispatch_before_agent_start_hook()` / `dispatch_after_llm_call_hook()`

Dispatches hook registry events for observability and customization.

## model/types.rs: LLM Provider Interface

**Source:** `crates/agents/src/model/types.rs`

### LlmProvider Trait

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse>;
    async fn chat_stream(&self, request: ChatRequest) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent>>>>>;
    fn supports_tools(&self) -> bool;
    fn model_id(&self) -> &str;
}
```

### ChatMessage

```rust
pub enum ChatMessage {
    System { content: String },
    User { content: UserContent },
    Assistant { content: String, tool_calls: Vec<ToolCall> },
    Tool { tool_call_id: String, content: String },
}
```

### ToolCall

```rust
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,  // JSON string
}
```

### StreamEvent

```rust
pub enum StreamEvent {
    TextDelta { text: String },
    ToolCallStart { tool_call: ToolCall },
    ToolCallDelta { tool_call_id: String, arguments: String },
    ToolCallEnd { tool_call: ToolCall },
    Usage { usage: Usage },
    RawEvent { event: String },
}
```

## tool_registry.rs: Tool Registration

**Source:** `crates/agents/src/tool_registry.rs` (~200 lines)

```rust
pub struct ToolRegistry {
    tools: HashMap<String, ToolDefinition>,
}

impl ToolRegistry {
    pub fn register(&mut self, tool: ToolDefinition);
    pub fn get(&self, name: &str) -> Option<&ToolDefinition>;
    pub fn list(&self) -> Vec<&ToolDefinition>;
    pub fn execute(&self, name: &str, args: &str) -> Result<String>;
}
```

## tool_parsing.rs: Tool Call Parsing

**Source:** `crates/agents/src/tool_parsing.rs` (~200 lines)

Parses tool calls from LLM response text when the provider doesn't support native tool calling:

```rust
pub fn parse_tool_calls_from_text(text: &str) -> Vec<ToolCall>;
pub fn looks_like_failed_tool_call(text: &str) -> bool;
```

## tool_loop_detector.rs: Infinite Loop Detection

**Source:** `crates/agents/src/tool_loop_detector.rs` (~150 lines)

```rust
pub struct ToolLoopDetector {
    recent_calls: VecDeque<ToolCallFingerprint>,
    max_window: usize,
}

impl ToolLoopDetector {
    pub fn is_loop(&mut self, fingerprint: &ToolCallFingerprint) -> bool;
}
```

Detects loops by:
1. Creating a fingerprint for each tool call (name + hashed arguments)
2. Checking if the same fingerprint appears N times in the recent window
3. If yes → loop detected

## Next

Continue to [06-providers-deep](06-providers-deep.md) for the LLM provider implementations deep dive.
