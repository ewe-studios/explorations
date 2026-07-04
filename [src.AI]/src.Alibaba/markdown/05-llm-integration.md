---
title: Page-Agent LLM Integration — OpenAI Client, Retry Logic, Tool Calling
---

# Page-Agent LLM Integration — OpenAI Client, Retry Logic, Tool Calling

**Page-Agent's LLM layer abstracts any OpenAI-compatible API behind a clean `LLM.invoke()` interface.** The client handles request construction, tool calling with union schemas, response normalization, and automatic retry with configurable backoff. By default it uses the OpenAI chat completions format, but any provider with a compatible API works.

Source: `packages/llms/src/index.ts`, `packages/llms/src/OpenAIClient.ts`, `packages/llms/src/types.ts`

## LLMConfig

```typescript
// packages/llms/src/types.ts
export interface LLMConfig {
    baseURL: string          // API endpoint (e.g., https://api.openai.com/v1)
    model: string            // Model name (e.g., gpt-4o, claude-3-opus)
    apiKey?: string          // API key (default: '')
    temperature?: number     // Default: 0 (deterministic)
    maxRetries?: number      // Default: 3
    transformRequestBody?: (body: object) => object  // Custom request mutation
    disableNamedToolChoice?: boolean   // Let model choose tools freely
    customFetch?: typeof fetch         // Custom HTTP client
}
```

Config validation in `parseLLMConfig()`:

```typescript
// packages/llms/src/index.ts:9-28
export function parseLLMConfig(config: LLMConfig): Required<LLMConfig> {
    if (!config.baseURL || !config.model) {
        throw new Error(
            '[PageAgent] LLM configuration required. Please provide: baseURL, model.'
        )
    }
    return {
        temperature: config.temperature ?? 0,        // Deterministic by default
        maxRetries: config.maxRetries ?? 3,
        apiKey: config.apiKey || '',
        disableNamedToolChoice: config.disableNamedToolChoice ?? false,
        transformRequestBody: config.transformRequestBody ?? ((body) => body),
        customFetch: (config.customFetch ?? fetch).bind(globalThis),
    }
}
```

## invoke() Method

```typescript
// packages/llms/src/index.ts:47-63
async invoke(
    messages: Message[],
    tools: Record<string, Tool>,
    abortSignal: AbortSignal,
    options?: InvokeOptions
): Promise<InvokeResult> {
    return await withRetry(
        async () => this.client.invoke(messages, tools, abortSignal, options),
        {
            maxRetries: this.config.maxRetries,
            onRetry: (attempt, lastError) => {
                this.dispatchEvent(new CustomEvent('retry', {
                    detail: { attempt, maxAttempts: this.config.maxRetries, lastError },
                }))
            },
        }
    )
}
```

## Retry Logic

```typescript
// packages/llms/src/index.ts:69-92
async function withRetry<T>(
    fn: () => Promise<T>,
    settings: { maxRetries: number; onRetry: (attempt: number, lastError: Error) => void }
): Promise<T> {
    let attempt = 0
    while (true) {
        try {
            return await fn()
        } catch (error) {
            if ((error as any)?.name === 'AbortError') throw error     // Never retry aborts
            if (error instanceof InvokeError && !error.retryable) throw error  // Non-retryable
            attempt++
            if (attempt > settings.maxRetries) throw error
            settings.onRetry(attempt, error as Error)
            await new Promise((resolve) => setTimeout(resolve, 100))  // 100ms retry delay
        }
    }
}
```

**Aha:** Retry only happens for retryable errors — rate limits (429), server errors (5xx), and transient network failures. `InvokeError` has a `retryable` boolean that the `OpenAIClient` sets based on the HTTP status code. Non-retryable errors (4xx auth failures, invalid requests) throw immediately without consuming retry budget. Abort errors (from `stop()`) also skip retry.

### InvokeError

```typescript
// packages/llms/src/errors.ts
export class InvokeError extends Error {
    retryable: boolean
    rawResponse?: unknown
}

export const InvokeErrorTypes = {
    NETWORK: 'network',       // fetch() threw — always retryable
    API_ERROR: 'api_error',   // 4xx/5xx — retryable depends on status
    PARSE_ERROR: 'parse_error', // JSON parse failure — retryable
    VALIDATION: 'validation',   // Response didn't match schema — retryable
}
```

## OpenAI Client

```typescript
// packages/llms/src/OpenAIClient.ts
class OpenAIClient implements LLMClient {
    async invoke(messages, tools, abortSignal, options): Promise<InvokeResult> {
        const requestBody = this.config.transformRequestBody({
            model: this.config.model,
            messages,
            tools: this.formatTools(tools),
            temperature: this.config.temperature,
            ...(options?.toolChoiceName && {
                tool_choice: { type: 'function', function: { name: options.toolChoiceName } }
            }),
        })

        const response = await this.config.customFetch(this.config.baseURL + '/chat/completions', {
            method: 'POST',
            headers: { 'Authorization': `Bearer ${this.config.apiKey}`, 'Content-Type': 'application/json' },
            body: JSON.stringify(requestBody),
            signal: abortSignal,
        })

        // Parse response, extract tool call, normalize with options.normalizeResponse
        // Return InvokeResult { toolResult, usage, rawResponse, rawRequest }
    }
}
```

## Custom Request Transformation

The `transformRequestBody` hook lets you adapt the request for non-OpenAI providers:

```typescript
const agent = new PageAgent({
    baseURL: 'https://api.anthropic.com/v1',
    model: 'claude-sonnet-4-20250514',
    apiKey: '...',
    transformRequestBody: (body) => ({
        ...body,
        max_tokens: 4096,                // Anthropic requires this
        // Anthropic doesn't use tool_choice.name the same way
    }),
})
```

## InvokeResult

```typescript
export interface InvokeResult {
    toolResult: {                     // The tool output
        input: Record<string, any>    // Parsed tool input (validated by schema)
        output: string                // Tool execution result
    }
    usage: {
        promptTokens: number
        completionTokens: number
        totalTokens: number
        cachedTokens?: number          // Anthropic/DashScope prompt caching
        reasoningTokens?: number       // DeepSeek R1 / o1 reasoning
    }
    rawResponse?: unknown             // For debugging
    rawRequest?: unknown              // For debugging
}
```

## Cross-References

- [Architecture](01-architecture.md) — How LLM fits in the dependency graph
- [Agent Loop](02-agent-loop.md) — How invoke() is called each step

## Next

→ [Extension & MCP](06-extension-mcp.md) — Browser extension, multi-page agent, and MCP server.
