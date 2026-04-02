# free-code API Provider Deep-Dive

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/free-code`

A comprehensive exploration of the multi-provider API architecture in free-code.

---

## Table of Contents

1. [Overview](#overview)
2. [Provider Architecture](#provider-architecture)
3. [Anthropic Provider](#anthropic-provider)
4. [OpenAI Codex Provider](#openai-codex-provider)
5. [AWS Bedrock Provider](#aws-bedrock-provider)
6. [Google Vertex Provider](#google-vertex-provider)
7. [Anthropic Foundry Provider](#anthropic-foundry-provider)
8. [Provider Selection Logic](#provider-selection-logic)
9. [Response Streaming](#response-streaming)
10. [Error Handling](#error-handling)
11. [Token Tracking](#token-tracking)

---

## Overview

free-code supports **five API providers** through a unified abstraction layer:

```
┌─────────────────────────────────────────────────────────────┐
│                    Application Layer                         │
│                   (QueryEngine, REPL)                        │
└─────────────────────────┬───────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                   Provider Abstraction                       │
│                   (getAPIProvider())                         │
└─────────────────────────┬───────────────────────────────────┘
                          │
          ┌───────────────┼───────────────┐
          │               │               │
          ▼               ▼               ▼
    ┌──────────┐   ┌──────────┐   ┌──────────┐
    │ Anthropic│   │  Codex   │   │ Bedrock  │
    │  (1P)    │   │ (OpenAI) │   │  (AWS)   │
    └──────────┘   └──────────┘   └──────────┘
          │               │
          ▼               ▼
    ┌──────────┐   ┌──────────┐
    │  Vertex  │   │ Foundry  │
    │ (Google) │   │(Anthropic)
    └──────────┘   └──────────┘
```

---

## Provider Architecture

### Provider Detection

Location: `src/utils/model/providers.ts`

```typescript
export type APIProvider = 
  | 'firstParty'    // Direct Anthropic API
  | 'bedrock'       // AWS Bedrock
  | 'vertex'        // Google Vertex AI
  | 'foundry'       // Anthropic Foundry
  | 'openai'        // OpenAI Codex

export function getAPIProvider(): APIProvider {
  return isEnvTruthy(process.env.CLAUDE_CODE_USE_BEDROCK)
    ? 'bedrock'
    : isEnvTruthy(process.env.CLAUDE_CODE_USE_VERTEX)
      ? 'vertex'
      : isEnvTruthy(process.env.CLAUDE_CODE_USE_FOUNDRY)
        ? 'foundry'
        : isEnvTruthy(process.env.CLAUDE_CODE_USE_OPENAI)
          ? 'openai'
          : 'firstParty'
}
```

**Environment Variable Priority:**
```
CLAUDE_CODE_USE_BEDROCK=1    → bedrock
CLAUDE_CODE_USE_VERTEX=1     → vertex
CLAUDE_CODE_USE_FOUNDRY=1    → foundry
CLAUDE_CODE_USE_OPENAI=1     → openai
(none set)                   → firstParty (Anthropic)
```

### Provider SDK Mapping

| Provider | SDK Package |
|----------|-------------|
| Anthropic | `@anthropic-ai/sdk` |
| AWS Bedrock | `@anthropic-ai/bedrock-sdk` |
| Google Vertex | `@anthropic-ai/vertex-sdk` |
| Anthropic Foundry | `@anthropic-ai/foundry-sdk` |
| OpenAI Codex | Custom adapter (`codex-fetch-adapter.ts`) |

---

## Anthropic Provider

### Configuration

```bash
# Environment variables
export ANTHROPIC_API_KEY="sk-ant-..."
export ANTHROPIC_BASE_URL="https://api.anthropic.com"  # Optional
export ANTHROPIC_MODEL="claude-opus-4-6"               # Optional
```

### Client Implementation

Location: `src/services/api/claude.ts` (126KB)

```typescript
import Anthropic from '@anthropic-ai/sdk'

const client = new Anthropic({
  apiKey: process.env.ANTHROPIC_API_KEY,
  baseURL: process.env.ANTHROPIC_BASE_URL,
  defaultHeaders: {
    'anthropic-version': '2023-06-01',
  },
})

export async function* streamMessages(params: MessagesParams) {
  const stream = client.messages.create({
    model: params.model,
    max_tokens: params.maxTokens,
    system: params.systemPrompt,
    messages: params.messages,
    tools: params.tools,
    stream: true,
  })

  for await (const event of stream) {
    yield formatEvent(event)
  }
}
```

### Supported Models

| Model | ID | Context Window | Pricing (input/output) |
|-------|-----|----------------|----------------------|
| Claude Opus 4.6 | `claude-opus-4-6` | 200K | $15 / $75 per 1M |
| Claude Sonnet 4.6 | `claude-sonnet-4-6` | 200K | $3 / $15 per 1M |
| Claude Haiku 4.5 | `claude-haiku-4-5` | 200K | $0.25 / $1.25 per 1M |

### Anthropic-Specific Features

**Thinking Mode:**
```typescript
{
  model: 'claude-opus-4-6',
  thinking: {
    type: 'enabled',
    budget_tokens: 10000,
  },
}
```

**Prompt Caching:**
```typescript
{
  messages: [{
    role: 'user',
    content: [
      {
        type: 'text',
        text: 'Long context...',
        cache_control: { type: 'ephemeral' },
      },
    ],
  }],
}
```

---

## OpenAI Codex Provider

### Configuration

```bash
# Enable Codex provider
export CLAUDE_CODE_USE_OPENAI=1

# Authenticate via OAuth
free-code /login
```

### Adapter Implementation

Location: `src/services/api/codex-fetch-adapter.ts` (28KB)

```typescript
export async function* adaptCodexResponse(
  codexStream: AsyncIterable<CodexEvent>
): AsyncGenerator<AnthropicStreamEvent> {
  for await (const event of codexStream) {
    // Translate reasoning → thinking
    if (event.type === 'response.reasoning.delta') {
      yield {
        type: 'content_block_delta',
        content_block_index: 0,
        delta: {
          type: 'thinking_delta',
          thinking: event.delta.text,
        },
      }
    }

    // Translate function_call_output → tool_result
    if (event.type === 'response.function_call_output') {
      yield {
        type: 'content_block_delta',
        content_block_index: 1,
        delta: {
          type: 'tool_result_delta',
          tool_use_id: event.call_id,
          content: event.output,
        },
      }
    }

    // Translate output_text → text_delta
    if (event.type === 'response.output_text.delta') {
      yield {
        type: 'content_block_delta',
        content_block_index: 0,
        delta: {
          type: 'text_delta',
          text: event.delta.text,
        },
      }
    }
  }

  // Handle completion with token usage
  if (event.type === 'response.completed') {
    yield {
      type: 'message_stop',
      usage: {
        input_tokens: event.response.usage.input_tokens,
        output_tokens: event.response.usage.output_tokens,
      },
    }
  }
}
```

### Key Translation Features

**1. Vision Translation:**
```typescript
// Anthropic format
{
  type: 'image',
  source: {
    type: 'base64',
    media_type: 'image/png',
    data: 'iVBORw0KG...',
  },
}

// Translated to Codex format
{
  type: 'input_image',
  image_url: 'data:image/png;base64,iVBORw0KG...',
}
```

**2. Cache Stripping:**
```typescript
// Remove Anthropic-only annotations
function stripCacheAnnotations(content: any): any {
  if (content.cache_control) {
    delete content.cache_control
  }
  return content
}
```

**3. Tool Result Routing:**
```typescript
// Anthropic: tool_result in content array
// Codex: function_call_output at top level
{
  output: [
    {
      type: 'function_call_output',
      call_id: 'call_abc123',
      output: 'Tool result here',
    },
  ],
}
```

### Supported Codex Models

| Model | ID | Best For |
|-------|-----|----------|
| GPT-5.3 Codex | `gpt-5.3-codex` | Code generation |
| GPT-5.4 | `gpt-5.4` | Complex reasoning |
| GPT-5.4 Mini | `gpt-5.4-mini` | Fast tasks |

---

## AWS Bedrock Provider

### Configuration

```bash
export CLAUDE_CODE_USE_BEDROCK=1
export AWS_REGION="us-east-1"
# Credentials via ~/.aws/credentials or IAM role
```

### SDK Usage

```typescript
import { AnthropicBedrock } from '@anthropic-ai/bedrock-sdk'

const client = new AnthropicBedrock({
  awsAccessKey: process.env.AWS_ACCESS_KEY_ID,
  awsSecretKey: process.env.AWS_SECRET_ACCESS_KEY,
  awsRegion: process.env.AWS_REGION,
})
```

### Model ARN Mapping

Models are automatically mapped to Bedrock ARN format:

```typescript
const MODEL_ARN_MAP: Record<string, string> = {
  'claude-opus-4-6': 'us.anthropic.claude-opus-4-6-v1',
  'claude-sonnet-4-6': 'us.anthropic.claude-sonnet-4-6-v1',
  'claude-haiku-4-5': 'us.anthropic.claude-haiku-4-5-v1',
}

// Custom region support
const REGION_PREFIX = process.env.AWS_REGION?.startsWith('eu-') ? 'eu' : 'us'
```

### Bedrock-Specific Features

**Custom Endpoint:**
```bash
export ANTHROPIC_BEDROCK_BASE_URL="https://bedrock.us-east-1.amazonaws.com"
```

**Skip Auth (Testing):**
```bash
export CLAUDE_CODE_SKIP_BEDROCK_AUTH=1
```

**Bearer Token Auth:**
```bash
export AWS_BEARER_TOKEN_BEDROCK="..."
```

---

## Google Vertex Provider

### Configuration

```bash
export CLAUDE_CODE_USE_VERTEX=1
gcloud auth application-default login
```

### SDK Usage

```typescript
import { AnthropicVertex } from '@anthropic-ai/vertex-sdk'

const client = new AnthropicVertex({
  projectId: process.env.GOOGLE_CLOUD_PROJECT,
  region: process.env.GOOGLE_CLOUD_REGION || 'us-central1',
})
```

### Model Mapping

```typescript
const VERTEX_MODEL_MAP: Record<string, string> = {
  'claude-opus-4-6': 'claude-opus-4-6@latest',
  'claude-sonnet-4-6': 'claude-sonnet-4-6@latest',
  'claude-haiku-4-5': 'claude-haiku-4-5@latest',
}
```

### Vertex-Specific Features

**Custom Region:**
```bash
export GOOGLE_CLOUD_REGION="us-central1"
```

**Service Account:**
```bash
export GOOGLE_APPLICATION_CREDENTIALS="/path/to/service-account.json"
```

---

## Anthropic Foundry Provider

### Configuration

```bash
export CLAUDE_CODE_USE_FOUNDRY=1
export ANTHROPIC_FOUNDRY_API_KEY="..."
export ANTHROPIC_FOUNDRY_BASE_URL="https://foundry.anthropic.com"  # Optional
```

### SDK Usage

```typescript
import { AnthropicFoundry } from '@anthropic-ai/foundry-sdk'

const client = new AnthropicFoundry({
  apiKey: process.env.ANTHROPIC_FOUNDRY_API_KEY,
  baseURL: process.env.ANTHROPIC_FOUNDRY_BASE_URL,
})
```

### Deployment IDs

Foundry uses **deployment IDs** as model names:

```bash
# Use your deployment ID as the model
free-code --model "my-deployment-id"
```

---

## Provider Selection Logic

### Selection Flow

```
User Request
    │
    ▼
┌─────────────────────────┐
│ getAPIProvider()        │
│ Check env variables     │
└───────────┬─────────────┘
            │
    ┌───────┴───────┐
    │               │
    ▼               ▼
┌──────────┐  ┌──────────┐
│ OpenAI   │  │ Bedrock  │
│ CLAUDE_  │  │ CLAUDE_  │
│ CODE_USE_│  │ CODE_USE_│
│ OPENAI=1 │  │ BEDROCK=1│
└──────────┘  └──────────┘
    │               │
    ▼               ▼
┌──────────┐  ┌──────────┐
│ Vertex   │  │ Foundry  │
│ CLAUDE_  │  │ CLAUDE_  │
│ CODE_USE_│  │ CODE_USE_│
│ VERTEX=1 │  │ FOUNDRY=1│
└──────────┘  └──────────┘
            │
            ▼
      ┌──────────┐
      │ Default: │
      │ Anthropic│
      │ (1P)     │
      └──────────┘
```

### Runtime Provider Switching

```typescript
// In QueryEngine.ts
import { getAPIProvider } from '../utils/model/providers.js'

async function executeQuery(params: QueryParams) {
  const provider = getAPIProvider()
  
  switch (provider) {
    case 'openai':
      return executeCodexQuery(params)
    case 'bedrock':
      return executeBedrockQuery(params)
    case 'vertex':
      return executeVertexQuery(params)
    case 'foundry':
      return executeFoundryQuery(params)
    default:
      return executeAnthropicQuery(params)
  }
}
```

---

## Response Streaming

### SSE Format (Anthropic)

```
event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}

event: content_block_stop
data: {"type":"content_block_stop","index":0}

event: message_stop
data: {"type":"message_stop","usage":{"input_tokens":100,"output_tokens":50}}
```

### Codex Event Format

```json
{
  "type": "response.output_text.delta",
  "delta": {
    "text": "Hello"
  }
}

{
  "type": "response.reasoning.delta",
  "delta": {
    "text": "Let me think about this..."
  }
}

{
  "type": "response.completed",
  "response": {
    "usage": {
      "input_tokens": 100,
      "output_tokens": 50
    }
  }
}
```

### Stream Parser Implementation

```typescript
async function* parseSSE(stream: ReadableStream) {
  const reader = stream.getReader()
  const decoder = new TextDecoder()
  let buffer = ''

  while (true) {
    const { done, value } = await reader.read()
    if (done) break

    buffer += decoder.decode(value, { stream: true })
    
    const lines = buffer.split('\n')
    buffer = lines.pop() || ''

    for (const line of lines) {
      if (line.startsWith('data: ')) {
        const data = JSON.parse(line.slice(6))
        yield data
      }
    }
  }
}
```

---

## Error Handling

### Error Types

```typescript
// src/services/api/errors.ts (42KB)

export class APIError extends Error {
  constructor(
    public status: number,
    public code: string,
    message: string,
  ) {
    super(message)
  }
}

export class RateLimitError extends APIError {
  constructor(
    message: string,
    public retryAfter?: number,
  ) {
    super(429, 'rate_limit_error', message)
  }
}

export class AuthenticationError extends APIError {
  constructor(message: string) {
    super(401, 'authentication_error', message)
  }
}
```

### Retry Logic

Location: `src/services/api/withRetry.ts` (28KB)

```typescript
export async function withRetry<T>(
  fn: () => Promise<T>,
  options: RetryOptions = {}
): Promise<T> {
  const {
    maxAttempts = 3,
    initialDelay = 1000,
    maxDelay = 30000,
  } = options

  let lastError: Error

  for (let attempt = 1; attempt <= maxAttempts; attempt++) {
    try {
      return await fn()
    } catch (error) {
      lastError = error as Error
      
      if (!isRetryableError(error) || attempt === maxAttempts) {
        throw error
      }

      const delay = calculateDelay(initialDelay, maxDelay, attempt)
      await sleep(delay)
    }
  }

  throw lastError!
}

function isRetryableError(error: unknown): boolean {
  if (error instanceof APIError) {
    return error.status >= 500 || error.status === 429
  }
  return false
}
```

### Provider-Specific Errors

**Anthropic:**
```typescript
{
  "type": "error",
  "error": {
    "type": "rate_limit_error",
    "message": "Rate limit exceeded"
  }
}
```

**Codex:**
```typescript
{
  "error": {
    "code": "rate_limit_exceeded",
    "message": "Rate limit exceeded",
    "status": 429
  }
}
```

---

## Token Tracking

### Token Usage Collection

```typescript
// In stream handler
if (event.type === 'message_stop') {
  const { input_tokens, output_tokens } = event.usage
  
  // Track for cost calculation
  trackTokenUsage({
    input: input_tokens,
    output: output_tokens,
    model: currentModel,
    provider: getAPIProvider(),
  })

  // Display to user
  displayTokenSummary({
    input: formatTokens(input_tokens),
    output: formatTokens(output_tokens),
    cost: calculateCost(input_tokens, output_tokens, currentModel),
  })
}
```

### Cost Calculation

```typescript
// src/utils/modelCost.ts

const MODEL_COSTS: Record<string, { input: number; output: number }> = {
  'claude-opus-4-6': { input: 15, output: 75 },      // per 1M tokens
  'claude-sonnet-4-6': { input: 3, output: 15 },
  'claude-haiku-4-5': { input: 0.25, output: 1.25 },
  'gpt-5.3-codex': { input: 2, output: 8 },
  'gpt-5.4': { input: 5, output: 20 },
}

export function calculateCost(
  inputTokens: number,
  outputTokens: number,
  model: string
): number {
  const costs = MODEL_COSTS[model] || MODEL_COSTS['claude-sonnet-4-6']
  return (
    (inputTokens / 1_000_000) * costs.input +
    (outputTokens / 1_000_000) * costs.output
  )
}
```

### Token Budget Tracking

```typescript
// src/utils/tokenBudget.ts

export function parseTokenBudget(budget?: string): number | undefined {
  if (!budget) return undefined
  
  const match = budget.match(/^(\d+(?:\.\d+)?)\s*(K|M)?$/i)
  if (!match) return undefined
  
  const value = parseFloat(match[1])
  const unit = match[2]?.toUpperCase()
  
  return unit === 'M' ? value * 1_000_000 : 
         unit === 'K' ? value * 1_000 : value
}

export function checkTokenBudget(
  usedTokens: number,
  budget?: number
): BudgetStatus {
  if (!budget) return { status: 'unlimited' }
  
  const remaining = budget - usedTokens
  const percentage = (usedTokens / budget) * 100
  
  if (remaining <= 0) return { status: 'exceeded', remaining: 0 }
  if (percentage >= 90) return { status: 'warning', remaining, percentage }
  return { status: 'ok', remaining, percentage }
}
```

---

## References

- [src/services/api/claude.ts](/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/free-code/src/services/api/claude.ts) — Anthropic API client
- [src/services/api/codex-fetch-adapter.ts](/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/free-code/src/services/api/codex-fetch-adapter.ts) — Codex adapter
- [src/utils/model/providers.ts](/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/free-code/src/utils/model/providers.ts) — Provider detection
- [src/services/api/errors.ts](/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/free-code/src/services/api/errors.ts) — Error handling
- [src/services/api/withRetry.ts](/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/free-code/src/services/api/withRetry.ts) — Retry logic
- [changes.md](/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/free-code/changes.md) — Codex support PR details
