# Agents Deep Dive: The Streaming Agent Loop

The agents crate implements the core agent loop — the conversation engine that calls LLMs, executes tools, and manages the conversation flow. This document covers the actual implementation.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Moltis/moltis/crates/agents/src/`

## Crate Structure

```
crates/agents/src/
├── runner/
│   ├── mod.rs              ← Entry point, re-exports, SteerInbox
│   ├── streaming.rs        ← Streaming agent loop (~500 lines)
│   ├── non_streaming.rs    ← Non-streaming variant
│   ├── helpers.rs          ← Helper functions (~500 lines)
│   ├── retry.rs            ← Retry logic for rate limits
│   └── tool_result.rs      ← Tool result sanitization
├── model/
│   ├── mod.rs              ← Model types
│   ├── chat.rs             ← ChatMessage enum
│   ├── stream.rs           ← StreamEvent enum
│   ├── types.rs            ← LlmProvider trait, ToolCall, Usage
│   └── convert.rs          ← Message conversion
├── prompt/
│   ├── builder.rs          ← Prompt builder
│   ├── formatting.rs       ← Prompt formatting
│   └── types.rs            ← Prompt types
├── tool_registry.rs        ← Tool registration
├── tool_parsing.rs         ← Parse tool calls from text
├── tool_arg_validator.rs   ← Validate tool arguments
├── tool_loop_detector.rs   ← Detect infinite tool loops
├── response_sanitizer.rs   ← Sanitize LLM responses
└── json_repair.rs          ← Repair malformed JSON
```

## The Streaming Agent Loop

**Source:** `runner/streaming.rs`

### Entry Point

```rust
// runner/streaming.rs:57-83
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
    steer_inbox: Option<SteerInbox>,
) -> Result<AgentRunResult, AgentRunError>
```

Delegates to `run_agent_loop_streaming_with_limits()` with default `AgentLoopLimits`.

### Main Loop Logic

**Source:** `runner/streaming.rs:85-500+`

The loop flow:

1. **Initialize** (lines 97-181):
   - Load config from `moltis_config::discover_and_load()`
   - Get `max_tool_result_bytes`, `agent_max_auto_continues`, `compaction_ratio`, etc.
   - Resolve `max_iterations` — tripled for lazy mode (`config.tools.registry_mode == Lazy`)
   - Build initial messages: `[system_prompt] + history + user_message`
   - Create `ToolLoopDetector` with configured window size

2. **Loop iteration** (lines 182+):
   - Check iteration count against `max_iterations`
   - Build `schemas_for_api` — tool schemas for this turn
   - Enforce context budget via `enforce_tool_result_context_budget()`
   - Dispatch `BeforeLLMCall` hook (can block or modify payload)
   - Call `provider.stream_with_tools_and_options()`

3. **Stream processing** (lines 314+):
   ```rust
   while let Some(event) = stream.next().await {
       StreamEvent::Delta(text)     → accumulate answer text
       StreamEvent::ReasoningDelta(text) → accumulate reasoning
       StreamEvent::ToolCallStart{id, name, index} → start tool call
       StreamEvent::ToolCallArgumentsDelta{index, delta} → accumulate args
       StreamEvent::Done(usage)     → finalize usage tracking
   }
   ```

4. **Post-stream processing**:
   - Parse accumulated tool call arguments from JSON
   - Validate tool arguments via `validate_tool_args()`
   - Check for tool loops via `ToolCallFingerprint`
   - If loop detected: either strip tools or return error

5. **Tool execution**:
   - If tool calls found: execute each via `resolve_tool_lookup()`
   - Sanitize results via `sanitize_tool_result()`
   - Add tool result messages to conversation
   - Continue loop for next iteration

6. **Completion**:
   - If no tool calls and substantive answer: return `AgentRunResult`
   - If empty answer but previous had tool calls: use `last_answer_text`
   - If answer is empty: return error

### Key Constants and Config

| Config | Source | Default |
|--------|--------|---------|
| `max_tool_result_bytes` | `config.tools` | — |
| `agent_max_iterations` | `config.tools` | — |
| `agent_max_auto_continues` | `config.tools` | — |
| `tool_result_compaction_ratio` | `config.tools` | — |
| `compaction_min_iterations` | `config.tools` | — |
| `agent_loop_detector_window` | `config.tools` | — |
| `RATE_LIMIT_MAX_RETRIES` | `retry.rs` | — |

### Tool Loop Detection

**Source:** `tool_loop_detector.rs`

```rust
pub struct ToolLoopDetector {
    window: usize,
    strip_tools_on_second_fire: bool,
    recent_calls: VecDeque<ToolCallFingerprint>,
}
```

The detector tracks recent tool calls and detects when the same pattern repeats within the window. On first fire it warns; on second fire it can strip tools (force text-only response) or error.

### Context Budget Enforcement

**Source:** `runner/helpers.rs`

```rust
fn enforce_tool_result_context_budget(
    messages: &mut Vec<ChatMessage>,
    tool_schemas: &[serde_json::Value],
    context_window: usize,
    compaction_ratio: usize,
    overflow_ratio: usize,
) -> Result<(), AgentRunError>
```

Compacts old tool results when the context approaches the model's limit. Uses `compaction_ratio` to determine how aggressively to compact.

### Response Sanitization

**Source:** `response_sanitizer.rs`

```rust
pub fn clean_response(text: &str) -> (String, Vec<ToolCall>)
```

Cleans the LLM response text and recovers any tool calls that were embedded in the text rather than properly structured.

## The LlmProvider Trait

**Source:** `model/types.rs`

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn name(&self) -> &str;
    fn id(&self) -> &str;
    fn context_window(&self) -> usize;
    fn supports_tools(&self) -> bool;
    fn supports_multimodal(&self) -> bool;
    
    async fn stream_with_tools_and_options(
        &self,
        messages: Vec<ChatMessage>,
        tool_schemas: Vec<serde_json::Value>,
        tool_controls: AgentToolControls,
    ) -> Pin<Box<dyn Stream<Item = StreamEvent>>>;
}
```

## ChatMessage

**Source:** `model/chat.rs`

```rust
pub enum ChatMessage {
    System { content: String },
    User { content: UserContent, name: Option<String> },
    Assistant { content: String, tool_calls: Vec<ToolCall> },
    Tool { tool_call_id: String, content: String },
}
```

## StreamEvent

**Source:** `model/stream.rs`

```rust
pub enum StreamEvent {
    Delta(String),                    // Answer text
    ReasoningDelta(String),           // Reasoning text
    ToolCallStart { id, name, index, metadata },
    ToolCallArgumentsDelta { index, delta },
    ToolCallComplete { index },
    ProviderRaw(serde_json::Value),   // Raw provider event
    Done(Usage),
}
```

## ToolCall

```rust
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
    pub argument_diagnostic: Option<String>,
    pub metadata: Option<serde_json::Value>,
}
```

## Usage Tracking

```rust
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_read_tokens: u32,
    pub cache_write_tokens: u32,
}
```

Tracked via `UsageAccumulator` which sums across iterations.

## Retry Logic

**Source:** `runner/retry.rs`

```rust
pub const RATE_LIMIT_MAX_RETRIES: u8 = 3;

pub fn is_context_window_error(error: &str) -> bool;
pub fn next_retry_delay_ms(retry_count: u8) -> u64;
```

Retries on rate limits with exponential backoff, up to `RATE_LIMIT_MAX_RETRIES`.

## Next

Continue to [06-providers-deep](06-providers-deep.md) for the LLM provider implementations.
