# AI SDK Ex - Deep Dive

## Overview

**ai-sdk-ex** is a minimal Elixir AI SDK scaffolding for streaming text with tool calls. It provides a clean interface for streaming LLM responses with support for multiple providers (OpenAI, Anthropic, OpenRouter, OpenCode Zen).

---

## Package Structure

```
ai-sdk-ex/
├── lib/
│   └── ai.ex            # Main AI module
├── examples/
│   └── stream_text.exs  # Usage examples
├── mix.exs              # Mix configuration
└── README.md
```

---

## Core API

### Streaming Text

```elixir
case AI.stream_text(
       model: AI.OpenRouter.chat("anthropic/claude-haiku-4.5"),
       prompt: "Say hello in one short sentence.",
       max_tokens: 64
     ) do
  {:ok, stream} ->
    Enum.each(stream, fn
      {:text_delta, _id, text} -> IO.write(text)
      {:finish, _info} -> IO.puts("")
      _ -> :ok
    end)

  other ->
    IO.inspect(other, label: "stream_text")
end
```

### Tool Calling

```elixir
add_tool = %{
  description: "Add two numbers",
  parameters: %{
    type: :object,
    properties: %{
      a: %{type: :number},
      b: %{type: :number}
    },
    required: [:a, :b]
  }
}

case AI.stream_text(
       model: AI.Anthropic.messages("claude-haiku-4-5"),
       prompt: "Use the add tool to add 7 and 5.",
       tools: %{"add" => add_tool},
       tool_choice: :required,
       max_steps: 3,
       max_tokens: 64
     ) do
  {:ok, stream} ->
    Enum.each(stream, fn
      {:tool_call, call} ->
        IO.inspect(call, label: "tool_call")

      {:tool_result, result} ->
        IO.inspect(result, label: "tool_result")

      {:text_delta, _id, text} ->
        IO.write(text)

      {:finish, _info} ->
        IO.puts("")

      _ -> :ok
    end)
end
```

---

## Stream Events

The stream emits structured events:

```elixir
{:text_start, _id}       # Text generation started
{:text_delta, _id, text} # Text chunk received
{:text_end, _id}         # Text generation ended
{:tool_call, call}       # Tool invocation
{:tool_result, result}   # Tool result
{:finish, info}          # Stream completed
{:error, error}          # Error occurred
{:raw, _}                # Raw provider event
```

### Minimal Handler

```elixir
case AI.stream_text(opts) do
  {:ok, stream} ->
    Enum.each(stream, fn
      {:text_delta, _id, text} ->
        IO.write(text)

      {:finish, _info} ->
        IO.puts("")

      _ -> :ok
    end)
end
```

---

## Provider Support

### OpenAI

```elixir
AI.stream_text(
  model: AI.OpenAI.responses("gpt-5.1-codex"),
  prompt: "Hello!",
  max_tokens: 64
)
```

### Anthropic

```elixir
AI.stream_text(
  model: AI.Anthropic.messages("claude-haiku-4-5"),
  prompt: "Hello!",
  max_tokens: 64
)
```

### OpenRouter

```elixir
AI.stream_text(
  model: AI.OpenRouter.chat("anthropic/claude-haudev-4.5"),
  prompt: "Hello!",
  max_tokens: 64,
  referer: "https://example.com",
  title: "MyApp"
)
```

### OpenCode Zen

```elixir
AI.stream_text(
  model: AI.OpenAI.responses(
    "gpt-5.1-codex",
    base_url: "https://opencode.ai/zen/v1",
    api_key: System.get_env("OPENCODE_API_KEY")
  ),
  prompt: "Hello!"
)
```

---

## Configuration

### Environment Variables

```bash
# .env file
OPENAI_API_KEY=...
ANTHROPIC_API_KEY=...
OPENROUTER_API_KEY=...
OPENCODE_API_KEY=...
```

### Mix Configuration

```elixir
def deps do
  [
    {:ai_sdk_ex, "~> 0.1.2"}
  ]
end
```

---

## Provider Patterns

### Dynamic Provider Selection

```elixir
provider = String.to_atom(System.get_env("AI_PROVIDER") || "openrouter")

model = case provider do
  :anthropic -> AI.Anthropic.messages("claude-haiku-4-5")
  :openai -> AI.OpenAI.responses("gpt-4o")
  :openrouter -> AI.OpenRouter.chat("anthropic/claude-haiku-4-5")
end

AI.stream_text(model: model, prompt: prompt)
```

### Custom Base URLs

```elixir
AI.Anthropic.messages(
  "claude-haiku-4-5",
  base_url: "https://custom-api.com/v1",
  api_key: System.get_env("CUSTOM_API_KEY")
)
```

---

## Rust Implementation Considerations

For a Rust equivalent:

### Architecture

```
ai-sdk-rs/
├── src/
│   ├── lib.rs          # Main module
│   ├── providers/
│   │   ├── openai.rs   # OpenAI provider
│   │   ├── anthropic.rs # Anthropic provider
│   │   └── openrouter.rs # OpenRouter provider
│   ├── stream.rs       # Streaming types
│   ├── tool.rs         # Tool definitions
│   └── error.rs        # Error types
└── Cargo.toml
```

### Core Types

```rust
pub enum StreamEvent {
    TextStart { id: String },
    TextDelta { id: String, text: String },
    TextEnd { id: String },
    ToolCall { tool: ToolCall },
    ToolResult { result: ToolResult },
    Finish { info: FinishInfo },
    Error { error: Error },
}

pub struct StreamEventIterator {
    // Internal state
}

impl Iterator for StreamEventIterator {
    type Item = StreamEvent;

    fn next(&mut self) -> Option<Self::Item> {
        // Stream processing
    }
}
```

### Provider Trait

```rust
pub trait Provider: Send + Sync {
    fn stream_text(
        &self,
        prompt: String,
        config: StreamConfig,
    ) -> Result<impl Stream<Item = StreamEvent>, Error>;

    fn name(&self) -> &str;
}
```

### Key Crates

- `reqwest` / `hyper` - HTTP client
- `tokio-stream` - Async streams
- `serde` - Serialization
- `thiserror` - Error handling
- `async-stream` - Stream builders
