# OpenAI Shim Deep Dive

**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/openclaude/src/services/api/openaiShim.ts`

**Lines of Code:** 724

---

## Overview

The OpenAI Shim is the core component that enables OpenClaude to work with any OpenAI-compatible API. It's a **translation layer** that:

1. Accepts calls in Anthropic SDK format
2. Converts them to OpenAI Chat Completions format
3. Sends requests to any OpenAI-compatible endpoint
4. Converts streaming responses back to Anthropic format
5. Returns data shaped like Anthropic SDK responses

### Why This Approach?

Instead of modifying hundreds of files throughout Claude Code to support multiple providers, the shim:

- **Isolates complexity** in a single 724-line file
- **Uses duck-typing** to present as Anthropic SDK
- **Requires zero changes** to the rest of the codebase
- **Adds zero dependencies** (uses native `fetch`)

---

## Architecture

### Position in Stack

```
┌─────────────────────────────────────┐
│      Claude Code Application        │
│  (Tools, Commands, Session Mgmt)    │
└─────────────────────────────────────┘
                  │
                  ▼ calls anthropic.beta.messages.create()
┌─────────────────────────────────────┐
│      @anthropic-ai/sdk             │
│    (Anthropic SDK interface)        │
└─────────────────────────────────────┘
                  │
                  ▼ when CLAUDE_CODE_USE_OPENAI=1
┌─────────────────────────────────────┐
│     OpenAI Shim (this file)         │
│  createOpenAIShimClient()           │
├─────────────────────────────────────┤
│  - convertMessages()                │
│  - convertTools()                   │
│  - openaiStreamToAnthropic()        │
│  - OpenAIShimMessages.create()      │
└─────────────────────────────────────┘
                  │
                  ▼ HTTP POST to /chat/completions
┌─────────────────────────────────────┐
│    OpenAI-Compatible API            │
│  (OpenAI, DeepSeek, Ollama, etc.)   │
└─────────────────────────────────────┘
```

### Class Structure

```typescript
// Main entry point
createOpenAIShimClient(options) → unknown (duck-typed as Anthropic)
    │
    ├── OpenAIShimBeta
    │   └── messages: OpenAIShimMessages
    │
    └── OpenAIShimMessages
        ├── create(params) → Promise | Stream
        ├── _doRequest(params) → Response
        └── _convertNonStreamingResponse(data) → AnthropicResponse

// Streaming support
OpenAIShimStream
    └── [Symbol.asyncIterator]() → AsyncGenerator<AnthropicStreamEvent>

// Stream conversion
openaiStreamToAnthropic(response, model) → AsyncGenerator<AnthropicStreamEvent>
```

---

## Message Format Conversion

### Anthropic vs OpenAI Formats

#### Anthropic Message Format

```typescript
interface AnthropicMessage {
  role: "user" | "assistant"
  content: string | Array<ContentBlock>
}

type ContentBlock =
  | { type: "text"; text: string }
  | { type: "image"; source: { type: "base64" | "url"; data: string; media_type: string } }
  | { type: "tool_use"; id: string; name: string; input: object }
  | { type: "tool_result"; tool_use_id: string; content: string | Array<ContentBlock> }
```

#### OpenAI Message Format

```typescript
interface OpenAIMessage {
  role: "system" | "user" | "assistant" | "tool"
  content: string | Array<{ type: "text" | "image_url"; text?: string; image_url?: { url: string } }>
  tool_calls?: Array<{
    id: string
    type: "function"
    function: { name: string; arguments: string }
  }>
  tool_call_id?: string  // For tool messages
  name?: string
}
```

### Conversion Algorithm

```typescript
function convertMessages(
  messages: Array<{ 
    role: string
    message?: { role?: string; content?: unknown }
    content?: unknown 
  }>,
  system: unknown,
): OpenAIMessage[] {
  const result: OpenAIMessage[] = []
  
  // 1. System message first (Anthropic sends separately)
  const sysText = convertSystemPrompt(system)
  if (sysText) {
    result.push({ role: "system", content: sysText })
  }
  
  // 2. Convert each message
  for (const msg of messages) {
    const inner = msg.message ?? msg
    const role = inner.role ?? msg.role
    const content = inner.content
    
    if (role === "user") {
      // Handle tool_result blocks → tool messages
      if (Array.isArray(content)) {
        const toolResults = content.filter(b => b.type === "tool_result")
        const otherContent = content.filter(b => b.type !== "tool_result")
        
        // Emit tool results as separate tool messages
        for (const tr of toolResults) {
          const trContent = Array.isArray(tr.content)
            ? tr.content.map(c => c.text ?? "").join("\n")
            : typeof tr.content === "string"
              ? tr.content
              : JSON.stringify(tr.content ?? "")
          
          result.push({
            role: "tool",
            tool_call_id: tr.tool_use_id ?? "unknown",
            content: tr.is_error ? `Error: ${trContent}` : trContent,
          })
        }
        
        // Emit remaining user content
        if (otherContent.length > 0) {
          result.push({
            role: "user",
            content: convertContentBlocks(otherContent),
          })
        }
      } else {
        result.push({
          role: "user",
          content: convertContentBlocks(content),
        })
      }
      
    } else if (role === "assistant") {
      // Handle tool_use blocks → assistant message with tool_calls
      if (Array.isArray(content)) {
        const toolUses = content.filter(b => b.type === "tool_use")
        const textContent = content.filter(
          b => b.type !== "tool_use" && b.type !== "thinking",
        )
        
        const assistantMsg: OpenAIMessage = {
          role: "assistant",
          content: convertContentBlocks(textContent) as string,
        }
        
        if (toolUses.length > 0) {
          assistantMsg.tool_calls = toolUses.map(tu => ({
            id: tu.id ?? `call_${Math.random().toString(36).slice(2)}`,
            type: "function",
            function: {
              name: tu.name ?? "unknown",
              arguments: typeof tu.input === "string"
                ? tu.input
                : JSON.stringify(tu.input ?? {}),
            },
          }))
        }
        
        result.push(assistantMsg)
      } else {
        result.push({
          role: "assistant",
          content: convertContentBlocks(content) as string,
        })
      }
    }
  }
  
  return result
}
```

### System Prompt Conversion

```typescript
function convertSystemPrompt(system: unknown): string {
  if (!system) return ""
  if (typeof system === "string") return system
  
  if (Array.isArray(system)) {
    // Anthropic sends system as array of text blocks
    return system
      .map((block: { type?: string; text?: string }) =>
        block.type === "text" ? block.text ?? "" : "",
      )
      .join("\n\n")
  }
  
  return String(system)
}
```

### Content Block Conversion

```typescript
function convertContentBlocks(
  content: unknown,
): string | Array<{ type: string; text?: string; image_url?: { url: string } }> {
  if (typeof content === "string") return content
  if (!Array.isArray(content)) return String(content ?? "")
  
  const parts: Array<{ type: string; text?: string; image_url?: { url: string } }> = []
  
  for (const block of content) {
    switch (block.type) {
      case "text":
        parts.push({ type: "text", text: block.text ?? "" })
        break
        
      case "image": {
        const src = block.source
        if (src?.type === "base64") {
          // Convert to data URL
          parts.push({
            type: "image_url",
            image_url: {
              url: `data:${src.media_type};base64,${src.data}`,
            },
          })
        } else if (src?.type === "url") {
          parts.push({
            type: "image_url",
            image_url: { url: src.url },
          })
        }
        break
      }
      
      case "tool_use":
        // Handled separately in convertMessages
        break
        
      case "tool_result":
        // Handled separately in convertMessages
        break
        
      case "thinking":
        // Append thinking as text with marker
        if (block.thinking) {
          parts.push({
            type: "text",
            text: `<thinking>${block.thinking}</thinking>`,
          })
        }
        break
        
      default:
        if (block.text) {
          parts.push({ type: "text", text: block.text })
        }
    }
  }
  
  if (parts.length === 0) return ""
  if (parts.length === 1 && parts[0].type === "text") return parts[0].text ?? ""
  return parts
}
```

---

## Tool Conversion

### Anthropic Tools → OpenAI Tools

```typescript
function convertTools(
  tools: Array<{
    name: string
    description?: string
    input_schema?: Record<string, unknown>
  }>,
): OpenAITool[] {
  return tools
    .filter(t => t.name !== "ToolSearchTool")  // Not relevant for OpenAI
    .map(t => ({
      type: "function" as const,
      function: {
        name: t.name,
        description: t.description ?? "",
        parameters: t.input_schema ?? { type: "object", properties: {} },
      },
    }))
}
```

### Tool Choice Conversion

```typescript
// Inside _doRequest()
if (params.tool_choice) {
  const tc = params.tool_choice as { type?: string; name?: string }
  
  if (tc.type === "auto") {
    body.tool_choice = "auto"
  } else if (tc.type === "tool" && tc.name) {
    body.tool_choice = {
      type: "function",
      function: { name: tc.name },
    }
  } else if (tc.type === "any") {
    body.tool_choice = "required"
  }
}
```

---

## Streaming Architecture

### OpenAI SSE Format

OpenAI streams Server-Sent Events (SSE) with this format:

```
data: {"id":"chatcmpl-abc","object":"chat.completion.chunk","choices":[{"delta":{"content":"Hello"},"finish_reason":null}]}

data: {"id":"chatcmpl-abc","object":"chat.completion.chunk","choices":[{"delta":{"content":"!"},"finish_reason":null}]}

data: {"id":"chatcmpl-abc","object":"chat.completion.chunk","choices":[{"delta":{},"finish_reason":"stop"}],"usage":{"completion_tokens":1}}

data: [DONE]
```

### Anthropic Stream Format

Anthropic expects these event types:

| Event | Description |
|-------|-------------|
| `message_start` | Beginning of response |
| `content_block_start` | Start of text/tool block |
| `content_block_delta` | Text/tool content chunk |
| `content_block_stop` | End of block |
| `message_delta` | Stop reason and usage |
| `message_stop` | End of response |

### Stream Conversion Implementation

```typescript
async function* openaiStreamToAnthropic(
  response: Response,
  model: string,
): AsyncGenerator<AnthropicStreamEvent> {
  const messageId = makeMessageId()
  let contentBlockIndex = 0
  const activeToolCalls = new Map<number, { id: string; name: string; index: number }>()
  let hasEmittedContentStart = false
  
  // 1. Emit message_start
  yield {
    type: "message_start",
    message: {
      id: messageId,
      type: "message",
      role: "assistant",
      content: [],
      model: model,
      stop_reason: null,
      stop_sequence: null,
      usage: {
        input_tokens: 0,
        output_tokens: 0,
        cache_creation_input_tokens: 0,
        cache_read_input_tokens: 0,
      },
    },
  }
  
  // 2. Read OpenAI SSE stream
  const reader = response.body?.getReader()
  if (!reader) return
  
  const decoder = new TextDecoder()
  let buffer = ""
  
  while (true) {
    const { done, value } = await reader.read()
    if (done) break
    
    buffer += decoder.decode(value, { stream: true })
    const lines = buffer.split("\n")
    buffer = lines.pop() ?? ""  // Keep incomplete line for next chunk
    
    for (const line of lines) {
      const trimmed = line.trim()
      if (!trimmed || trimmed === "data: [DONE]") continue
      if (!trimmed.startsWith("data: ")) continue
      
      let chunk: OpenAIStreamChunk
      try {
        chunk = JSON.parse(trimmed.slice(6))  // Remove "data: " prefix
      } catch {
        continue  // Skip malformed chunks
      }
      
      for (const choice of chunk.choices ?? []) {
        const delta = choice.delta
        
        // 2a. Text content
        if (delta.content) {
          if (!hasEmittedContentStart) {
            yield {
              type: "content_block_start",
              index: contentBlockIndex,
              content_block: { type: "text", text: "" },
            }
            hasEmittedContentStart = true
          }
          yield {
            type: "content_block_delta",
            index: contentBlockIndex,
            delta: { type: "text_delta", text: delta.content },
          }
        }
        
        // 2b. Tool calls
        if (delta.tool_calls) {
          for (const tc of delta.tool_calls) {
            if (tc.id && tc.function?.name) {
              // New tool call starting
              if (hasEmittedContentStart) {
                yield { type: "content_block_stop", index: contentBlockIndex }
                contentBlockIndex++
                hasEmittedContentStart = false
              }
              
              const toolBlockIndex = contentBlockIndex
              activeToolCalls.set(tc.index, {
                id: tc.id,
                name: tc.function.name,
                index: toolBlockIndex,
              })
              
              yield {
                type: "content_block_start",
                index: toolBlockIndex,
                content_block: {
                  type: "tool_use",
                  id: tc.id,
                  name: tc.function.name,
                  input: {},
                },
              }
              contentBlockIndex++
              
              // Emit initial arguments
              if (tc.function.arguments) {
                yield {
                  type: "content_block_delta",
                  index: toolBlockIndex,
                  delta: {
                    type: "input_json_delta",
                    partial_json: tc.function.arguments,
                  },
                }
              }
              
            } else if (tc.function?.arguments) {
              // Continuation of existing tool call
              const active = activeToolCalls.get(tc.index)
              if (active) {
                yield {
                  type: "content_block_delta",
                  index: active.index,
                  delta: {
                    type: "input_json_delta",
                    partial_json: tc.function.arguments,
                  },
                }
              }
            }
          }
        }
        
        // 2c. Finish
        if (choice.finish_reason) {
          // Close any open content blocks
          if (hasEmittedContentStart) {
            yield { type: "content_block_stop", index: contentBlockIndex }
          }
          
          // Close active tool calls
          for (const [, tc] of activeToolCalls) {
            yield { type: "content_block_stop", index: tc.index }
          }
          
          // Map finish_reason to stop_reason
          const stopReason =
            choice.finish_reason === "tool_calls"
              ? "tool_use"
              : choice.finish_reason === "length"
                ? "max_tokens"
                : "end_turn"
          
          yield {
            type: "message_delta",
            delta: { stop_reason: stopReason, stop_sequence: null },
            usage: {
              output_tokens: chunk.usage?.completion_tokens ?? 0,
            },
          }
        }
      }
    }
  }
  
  // 3. Emit message_stop
  yield { type: "message_stop" }
}
```

### Stream Wrapper Class

```typescript
class OpenAIShimStream {
  private generator: AsyncGenerator<AnthropicStreamEvent>
  // The controller property is checked by claude.ts to distinguish streams
  controller = new AbortController()
  
  constructor(generator: AsyncGenerator<AnthropicStreamEvent>) {
    this.generator = generator
  }
  
  async *[Symbol.asyncIterator]() {
    yield* this.generator
  }
}
```

---

## HTTP Request Handling

### Building the Request

```typescript
private async _doRequest(
  params: ShimCreateParams,
  options?: { signal?: AbortSignal; headers?: Record<string, string> },
): Promise<Response> {
  // 1. Convert messages
  const openaiMessages = convertMessages(
    params.messages as Array<{ role: string; content?: unknown }>,
    params.system,
  )
  
  // 2. Build request body
  const body: Record<string, unknown> = {
    model: params.model,
    messages: openaiMessages,
    max_tokens: params.max_tokens,
    stream: params.stream ?? false,
  }
  
  // 3. Add streaming options
  if (params.stream) {
    body.stream_options = { include_usage: true }
  }
  
  // 4. Add sampling parameters
  if (params.temperature !== undefined) body.temperature = params.temperature
  if (params.top_p !== undefined) body.top_p = params.top_p
  
  // 5. Convert and add tools
  if (params.tools && params.tools.length > 0) {
    const converted = convertTools(
      params.tools as Array<{ name: string; input_schema?: object }>,
    )
    if (converted.length > 0) {
      body.tools = converted
      
      // Convert tool_choice
      if (params.tool_choice) {
        const tc = params.tool_choice as { type?: string; name?: string }
        if (tc.type === "auto") {
          body.tool_choice = "auto"
        } else if (tc.type === "tool" && tc.name) {
          body.tool_choice = {
            type: "function",
            function: { name: tc.name },
          }
        } else if (tc.type === "any") {
          body.tool_choice = "required"
        }
      }
    }
  }
  
  // 6. Build URL and headers
  const url = `${this.baseUrl}/chat/completions`
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    ...this.defaultHeaders,
    ...(options?.headers ?? {}),
  }
  
  if (this.apiKey) {
    headers["Authorization"] = `Bearer ${this.apiKey}`
  }
  
  // 7. Make request
  const response = await fetch(url, {
    method: "POST",
    headers,
    body: JSON.stringify(body),
    signal: options?.signal,
  })
  
  if (!response.ok) {
    const errorBody = await response.text().catch(() => "unknown error")
    throw new Error(`OpenAI API error ${response.status}: ${errorBody}`)
  }
  
  return response
}
```

### Non-Streaming Response Conversion

```typescript
private _convertNonStreamingResponse(
  data: {
    id?: string
    model?: string
    choices?: Array<{
      message?: {
        role?: string
        content?: string | null
        tool_calls?: Array<{
          id: string
          function: { name: string; arguments: string }
        }>
      }
      finish_reason?: string
    }>
    usage?: { prompt_tokens?: number; completion_tokens?: number }
  },
  model: string,
) {
  const choice = data.choices?.[0]
  const content: Array<Record<string, unknown>> = []
  
  // 1. Extract text content
  if (choice?.message?.content) {
    content.push({ type: "text", text: choice.message.content })
  }
  
  // 2. Extract tool calls
  if (choice?.message?.tool_calls) {
    for (const tc of choice.message.tool_calls) {
      let input: unknown
      try {
        input = JSON.parse(tc.function.arguments)
      } catch {
        input = { raw: tc.function.arguments }
      }
      
      content.push({
        type: "tool_use",
        id: tc.id,
        name: tc.function.name,
        input,
      })
    }
  }
  
  // 3. Map finish_reason to stop_reason
  const stopReason =
    choice?.finish_reason === "tool_calls"
      ? "tool_use"
      : choice?.finish_reason === "length"
        ? "max_tokens"
        : "end_turn"
  
  // 4. Return Anthropic-format response
  return {
    id: data.id ?? makeMessageId(),
    type: "message",
    role: "assistant",
    content,
    model: data.model ?? model,
    stop_reason: stopReason,
    stop_sequence: null,
    usage: {
      input_tokens: data.usage?.prompt_tokens ?? 0,
      output_tokens: data.usage?.completion_tokens ?? 0,
      cache_creation_input_tokens: 0,
      cache_read_input_tokens: 0,
    },
  }
}
```

---

## Client Factory

### createOpenAIShimClient

```typescript
export function createOpenAIShimClient(options: {
  defaultHeaders?: Record<string, string>
  maxRetries?: number
  timeout?: number
}): unknown {
  // 1. Determine base URL
  const baseUrl = (
    process.env.OPENAI_BASE_URL ??
    process.env.OPENAI_API_BASE ??
    "https://api.openai.com/v1"
  ).replace(/\/+$/, "")  // Remove trailing slashes
  
  // 2. Get API key
  const apiKey = process.env.OPENAI_API_KEY ?? ""
  
  // 3. Build headers
  const headers = {
    ...(options.defaultHeaders ?? {}),
  }
  
  // 4. Create client hierarchy
  const beta = new OpenAIShimBeta(baseUrl, apiKey, headers)
  
  // 5. Return duck-typed as Anthropic client
  return {
    beta,
    // Some code paths access .messages directly (non-beta)
    messages: beta.messages,
  }
}
```

### Client Hierarchy

```typescript
class OpenAIShimBeta {
  messages: OpenAIShimMessages
  
  constructor(
    baseUrl: string,
    apiKey: string,
    defaultHeaders: Record<string, string>,
  ) {
    this.messages = new OpenAIShimMessages(baseUrl, apiKey, defaultHeaders)
  }
}

class OpenAIShimMessages {
  private baseUrl: string
  private apiKey: string
  private defaultHeaders: Record<string, string>
  
  constructor(
    baseUrl: string,
    apiKey: string,
    defaultHeaders: Record<string, string>,
  ) {
    this.baseUrl = baseUrl
    this.apiKey = apiKey
    this.defaultHeaders = defaultHeaders
  }
  
  create(params, options?) {
    // Returns Promise or Stream
  }
}
```

---

## Usage Examples

### Basic Usage

```typescript
// In client.ts
if (isEnvTruthy(process.env.CLAUDE_CODE_USE_OPENAI)) {
  const { createOpenAIShimClient } = await import('./openaiShim.js')
  return createOpenAIShimClient({
    defaultHeaders,
    maxRetries,
    timeout: parseInt(process.env.API_TIMEOUT_MS || String(600 * 1000), 10),
  }) as unknown as Anthropic
}
```

### Using the Client

```typescript
const client = createOpenAIShimClient({
  defaultHeaders: { "x-custom-header": "value" },
  maxRetries: 3,
})

// Streaming request
const stream = await client.beta.messages.create({
  model: "gpt-4o",
  messages: [{ role: "user", content: "Hello!" }],
  max_tokens: 1000,
  stream: true,
})

for await (const event of stream) {
  console.log(event.type, event)
}

// Non-streaming request
const response = await client.beta.messages.create({
  model: "gpt-4o",
  messages: [{ role: "user", content: "Hello!" }],
  max_tokens: 1000,
})

console.log(response.content[0].text)
```

---

## Provider Compatibility

### Tested Providers

| Provider | Base URL | Notes |
|----------|----------|-------|
| OpenAI | `https://api.openai.com/v1` | Full compatibility |
| DeepSeek | `https://api.deepseek.com/v1` | Full compatibility |
| Ollama | `http://localhost:11434/v1` | Full compatibility |
| LM Studio | `http://localhost:1234/v1` | Full compatibility |
| OpenRouter | `https://openrouter.ai/api/v1` | Full compatibility |
| Together | `https://api.together.xyz/v1` | Full compatibility |
| Groq | `https://api.groq.com/openai/v1` | Full compatibility |
| Mistral | `https://api.mistral.ai/v1` | Full compatibility |
| Azure OpenAI | `https://*.openai.azure.com/openai/...` | Full compatibility |

### Provider-Specific Notes

**Ollama:**
- No API key required
- Use `stream: true` for streaming
- Model names include tags (`llama3.3:70b`)

**Azure OpenAI:**
- URL format: `https://{resource}.openai.azure.com/openai/deployments/{deployment}/v1`
- API key required
- May need to adjust max_tokens based on model limits

**OpenRouter:**
- Model names include provider prefix (`google/gemini-2.0-flash`)
- API key from openrouter.ai

---

## Error Handling

### HTTP Errors

```typescript
const response = await fetch(url, { ... })

if (!response.ok) {
  const errorBody = await response.text().catch(() => "unknown error")
  throw new Error(`OpenAI API error ${response.status}: ${errorBody}`)
}
```

### Common Error Codes

| Status | Meaning | Action |
|--------|---------|--------|
| 400 | Bad Request | Check message format |
| 401 | Unauthorized | Check API key |
| 403 | Forbidden | Check permissions/quotas |
| 404 | Not Found | Check model name |
| 429 | Rate Limited | Implement backoff |
| 500 | Server Error | Retry with backoff |
| 503 | Unavailable | Retry with backoff |

---

## Performance Considerations

### Memory Efficiency

- Stream processing uses incremental parsing (no full buffer)
- Tool calls tracked in Map for O(1) lookups
- No unnecessary object allocations

### Latency

- First token latency depends on provider
- Streaming begins immediately (no buffering)
- Concurrent tool call tracking

### Throughput

- Single connection per request
- No connection pooling (relies on Node.js defaults)
- Consider adding pooling for high-throughput scenarios

---

## Extending the Shim

### Adding Custom Message Types

```typescript
function convertContentBlocks(content: unknown) {
  // Add support for new content types
  for (const block of content) {
    switch (block.type) {
      // ... existing cases
      
      case "custom_type":
        // Custom conversion logic
        parts.push({ type: "text", text: block.data })
        break
    }
  }
}
```

### Provider-Specific Headers

```typescript
const headers: Record<string, string> = {
  "Content-Type": "application/json",
  ...this.defaultHeaders,
}

// Add provider-specific headers
if (this.baseUrl.includes("azure")) {
  headers["api-key"] = this.apiKey
  delete headers["Authorization"]
}
```

---

## Testing

### Unit Tests

```typescript
describe("convertMessages", () => {
  it("converts simple user message", () => {
    const result = convertMessages(
      [{ role: "user", content: "Hello" }],
      undefined
    )
    expect(result).toEqual([{ role: "user", content: "Hello" }])
  })
  
  it("converts tool_use blocks", () => {
    const result = convertMessages(
      [{
        role: "assistant",
        content: [{
          type: "tool_use",
          id: "abc",
          name: "bash",
          input: { cmd: "ls" }
        }]
      }],
      undefined
    )
    expect(result[0].tool_calls).toBeDefined()
  })
})
```

### Integration Tests

```typescript
describe("OpenAIShimMessages", () => {
  it("streams from OpenAI", async () => {
    const client = createOpenAIShimClient({
      defaultHeaders: {},
    })
    
    const stream = await client.beta.messages.create({
      model: "gpt-4o-mini",
      messages: [{ role: "user", content: "Hi" }],
      stream: true,
    })
    
    const events = []
    for await (const event of stream) {
      events.push(event)
    }
    
    expect(events.length).toBeGreaterThan(0)
  })
})
```

---

## References

- [openaiShim.ts](/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/openclaude/src/services/api/openaiShim.ts) — Source code
- [01-openclaude-exploration.md](./01-openclaude-exploration.md) — Architecture overview
- [OpenAI API Reference](https://platform.openai.com/docs/api-reference/chat) — OpenAI docs
- [Anthropic API Reference](https://docs.anthropic.com/claude/reference/messages_post) — Anthropic docs
