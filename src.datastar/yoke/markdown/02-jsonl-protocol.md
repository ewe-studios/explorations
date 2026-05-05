# yoke -- JSONL Protocol

## Two Line Types

Every line of yoke's JSONL output is one of two types:

1. **Context lines** — Have a `role` field. Durable conversation state. Round-trip as input.
2. **Observation lines** — Have a `type` field. Live stream events. Skipped on input.

## Input Format

JSONL on stdin. Only lines with a `role` field are processed. Everything else is silently skipped (including observation lines from previous runs).

### System Message

```json
{"role": "system", "content": "You are a helpful coding assistant."}
```

### User Message (shorthand)

```json
{"role": "user", "content": "what files are here?"}
```

### User Message (full form)

```json
{"role": "user", "content": [{"type": "text", "text": "what files are here?"}], "timestamp": 1714924800000}
```

### Assistant Message

```json
{"role": "assistant", "content": [{"type": "text", "text": "I'll check..."}], "stopReason": "stop", "model": "claude-sonnet-4-20250514", "provider": "anthropic", "usage": {"inputTokens": 100, "outputTokens": 50}, "timestamp": 1714924801000}
```

### Tool Result Message

```json
{"role": "toolResult", "toolCallId": "call_123", "toolName": "list_files", "content": [{"type": "text", "text": "src/\nCargo.toml\nREADME.md"}], "isError": false, "timestamp": 1714924802000}
```

## Output Format

### Full Turn Example

```jsonl
{"type":"agent_start"}
{"role":"system","content":"You are a helpful coding assistant."}
{"type":"turn_start"}
{"role":"user","content":[{"type":"text","text":"what files are here?"}],"timestamp":1714924800000}
{"type":"delta","kind":"text","delta":"I'll list"}
{"type":"delta","kind":"text","delta":" the files."}
{"type":"delta","kind":"tool_call","delta":"{\"name\":\"list_files\"..."}
{"type":"tool_execution_start","tool_call_id":"call_abc","tool_name":"list_files","args":{"path":"."}}
{"type":"tool_execution_end","tool_call_id":"call_abc","tool_name":"list_files","result":{"content":[{"type":"text","text":"src/\nCargo.toml"}]},"is_error":false}
{"role":"toolResult","toolCallId":"call_abc","toolName":"list_files","content":[{"type":"text","text":"src/\nCargo.toml"}],"isError":false,"timestamp":1714924801000}
{"role":"assistant","content":[{"type":"toolCall","id":"call_abc","name":"list_files","arguments":{}}],"stopReason":"tool_use","model":"claude-sonnet-4-20250514","provider":"anthropic","usage":{"inputTokens":150,"outputTokens":30},"timestamp":1714924801000}
{"type":"turn_end"}
{"type":"turn_start"}
{"type":"delta","kind":"text","delta":"Here are the files:\n- src/\n- Cargo.toml"}
{"role":"assistant","content":[{"type":"text","text":"Here are the files:\n- src/\n- Cargo.toml"}],"stopReason":"stop","model":"claude-sonnet-4-20250514","provider":"anthropic","usage":{"inputTokens":200,"outputTokens":25},"timestamp":1714924802000}
{"type":"turn_end"}
{"type":"agent_end"}
```

## Content Types

```rust
enum Content {
    Text { text: String },
    Image { data: String, mime_type: String },
    Thinking { thinking: String, signature: Option<String> },
    ToolCall { id: String, name: String, arguments: serde_json::Value },
}
```

## Observation Line Types

| Type | Fields | Purpose |
|------|--------|---------|
| `agent_start` | — | Agent run beginning |
| `agent_end` | — | Agent run complete |
| `turn_start` | — | New LLM call starting |
| `turn_end` | — | LLM call + tool execution complete |
| `delta` | `kind`, `delta` | Streaming content chunk |
| `tool_execution_start` | `tool_call_id`, `tool_name`, `args` | Tool about to execute |
| `tool_execution_end` | `tool_call_id`, `tool_name`, `result`, `is_error` | Tool finished |
| `progress_message` | `tool_call_id`, `tool_name`, `text` | Tool progress update |
| `input_rejected` | `reason` | Input filter blocked the message |
| `tools` | `tools` | Emitted at start: list of available tools |

### Delta Kinds

| Kind | Content |
|------|---------|
| `text` | Model text output |
| `thinking` | Extended thinking (when enabled) |
| `tool_call` | Tool call JSON being constructed |

## Round-Tripping

The key insight: pipe output to a file, then pipe that file back as input for continuation.

```nushell
# Turn 1: save output
yoke --provider anthropic --model claude-sonnet-4-20250514 "refactor main.rs" \
  | tee { save -f session.jsonl }

# Turn 2: continue from saved state
cat session.jsonl \
  | yoke --provider anthropic --model claude-sonnet-4-20250514 "now add tests"
```

On input, yoke:
1. Reads all lines
2. Parses JSON
3. Keeps lines with `role` field (context)
4. Skips lines with `type` field (observations)
5. Skips lines that fail JSON parse
6. Extracts `system` role as the system prompt
7. Collects the rest as conversation messages

## Stop Reasons

```rust
enum StopReason {
    Stop,       // Natural completion
    ToolUse,    // Model wants to call tools (loop continues)
    MaxTokens,  // Output truncated
    Error,      // Provider error
}
```

## Usage Tracking

Every assistant message includes token usage:

```json
{
  "usage": {
    "inputTokens": 1500,
    "outputTokens": 300,
    "cacheReadTokens": 1200,
    "cacheWriteTokens": 300
  }
}
```
