# AI SDK Reasoning Starter - Deep Dive Exploration

## Overview

**AI SDK Reasoning Starter** is a Next.js chatbot template demonstrating integration with reasoning models like DeepSeek-R1 and Anthropic Claude 3.7 with thinking enabled.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AICoders/src.VarcelLabs/ai-sdk-reasoning-starter`

---

## Architecture

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  Next.js App    │ →── │  AI SDK          │ →── │  Model Providers │
│  (App Router)   │     │  (streamText)    │     │  - Anthropic     │
│                 │     │                  │     │  - Fireworks     │
│  /api/chat/route│     │  - reasoning     │     │  - Groq          │
└─────────────────┘     │  - streaming     │     └─────────────────┘
                        └──────────────────┘
```

---

## Tech Stack

| Component | Technology |
|-----------|------------|
| Framework | Next.js 15 (App Router) |
| UI | shadcn/ui + Radix UI + Tailwind |
| AI SDK | `ai` package (Vercel) |
| Models | Claude 3.7, DeepSeek-R1, Llama 70B |
| Streaming | `smoothStream` word-by-word |

---

## Key Implementation Details

### 1. Multi-Provider Setup (`lib/models.ts`)

```typescript
import { anthropic } from "@ai-sdk/anthropic";
import { fireworks } from "@ai-sdk/fireworks";
import { groq } from "@ai-sdk/groq";
import {
  customProvider,
  extractReasoningMiddleware,
  wrapLanguageModel,
} from "ai";

export const myProvider = customProvider({
  languageModels: {
    // Anthropic Claude 3.7 Sonnet
    "sonnet-3.7": anthropic("claude-3-7-sonnet-20250219"),

    // DeepSeek-R1 via Fireworks with reasoning extraction
    "deepseek-r1": wrapLanguageModel({
      middleware: extractReasoningMiddleware({
        tagName: "think",
      }),
      model: fireworks("accounts/fireworks/models/deepseek-r1"),
    }),

    // DeepSeek-R1 Distill via Groq
    "deepseek-r1-distill-llama-70b": wrapLanguageModel({
      middleware: extractReasoningMiddleware({
        tagName: "think",
      }),
      model: groq("deepseek-r1-distill-llama-70b"),
    }),
  },
});

export type modelID = Parameters<(typeof myProvider)["languageModel"]>["0"];

export const models: Record<modelID, string> = {
  "sonnet-3.7": "Claude Sonnet 3.7",
  "deepseek-r1": "DeepSeek-R1",
  "deepseek-r1-distill-llama-70b": "DeepSeek-R1 Llama 70B",
};
```

**Key Patterns:**

1. **`customProvider`** - Creates unified interface for multiple providers
2. **`wrapLanguageModel`** - Wraps models with middleware
3. **`extractReasoningMiddleware`** - Extracts <think> tags from reasoning models

### 2. Chat API Route (`app/api/chat/route.ts`)

```typescript
import { modelID, myProvider } from "@/lib/models";
import { convertToModelMessages, smoothStream, streamText, UIMessage } from "ai";
import { NextRequest } from "next/server";

export async function POST(request: NextRequest) {
  const {
    messages,
    selectedModelId,
    isReasoningEnabled,
  }: {
    messages: Array<UIMessage>;
    selectedModelId: modelID;
    isReasoningEnabled: boolean;
  } = await request.json();

  const stream = streamText({
    // Dynamic system prompt based on model
    system: selectedModelId === "deepseek-r1"
      ? "You are DeepSeek-R1, a reasoning model created by DeepSeek."
      : selectedModelId === "deepseek-r1-distill-llama-70b"
      ? "You are DeepSeek-R1 Llama 70B, a reasoning model created by DeepSeek."
      : "You are Claude, an AI assistant created by Anthropic.",

    // Anthropic-specific thinking configuration
    providerOptions:
      selectedModelId === "sonnet-3.7"
        ? {
            anthropic: {
              thinking: isReasoningEnabled
                ? { type: "enabled", budgetTokens: 12000 }
                : { type: "disabled", budgetTokens: 12000 },
            },
          }
        : {},

    model: myProvider.languageModel(selectedModelId),

    // Word-by-word streaming for smooth UX
    experimental_transform: [
      smoothStream({
        chunking: "word",
      }),
    ],

    messages: convertToModelMessages(messages),
  });

  return stream.toUIMessageStreamResponse({
    sendReasoning: true,  // Include reasoning in response
    onError: () => {
      return `An error occurred, please try again!`;
    },
  });
}
```

**Key Features:**

1. **`convertToModelMessages`** - Converts UI messages to AI SDK format
2. **`smoothStream`** - Chunks output by word for natural streaming
3. **`providerOptions`** - Provider-specific settings (Anthropic thinking budget)
4. **`sendReasoning: true`** - Includes <think> content in response

---

## Reasoning Model Integration

### Anthropic Claude 3.7 Thinking

```typescript
providerOptions: {
  anthropic: {
    thinking: isReasoningEnabled
      ? { type: "enabled", budgetTokens: 12000 }
      : { type: "disabled", budgetTokens: 12000 },
  },
}
```

**Options:**
- `type: "enabled"` - Model will think before responding
- `type: "disabled"` - Direct response (faster, less accurate)
- `budgetTokens: 12000` - Max tokens for thinking

### DeepSeek-R1 Reasoning Extraction

```typescript
"deepseek-r1": wrapLanguageModel({
  middleware: extractReasoningMiddleware({
    tagName: "think",
  }),
  model: fireworks("accounts/fireworks/models/deepseek-r1"),
}),
```

**How it works:**
1. DeepSeek-R1 outputs: `<think>...reasoning...</think>...answer...`
2. `extractReasoningMiddleware` parses and separates reasoning
3. Frontend receives both reasoning and answer separately

---

## AI SDK Patterns

### 1. Streaming Text Response

```typescript
const stream = streamText({
  model: model,
  messages: convertToModelMessages(messages),
});

return stream.toUIMessageStreamResponse();
```

### 2. Structured Output (not used in this project but related)

```typescript
import { generateObject } from 'ai';
import { z } from 'zod';

const { object } = await generateObject({
  model: 'openai/gpt-4',
  schema: z.object({
    name: z.string(),
    age: z.number(),
  }),
  prompt: 'Extract: John is 30 years old',
});
// object = { name: "John", age: 30 }
```

### 3. Tool Usage (not used in this project but related)

```typescript
import { tool } from 'ai';

const searchTool = tool({
  description: 'Search the web',
  inputSchema: z.object({ query: z.string() }),
  execute: async ({ query }) => {
    return await search(query);
  },
});

const stream = streamText({
  model: model,
  tools: { search: searchTool },
});
```

---

## Frontend Integration

### Chat Component Pattern

```typescript
// Client component using AI SDK hooks
import { useChat } from 'ai';

export function Chat() {
  const { messages, input, handleInputChange, handleSubmit } = useChat({
    api: '/api/chat',
    body: {
      selectedModelId: 'sonnet-3.7',
      isReasoningEnabled: true,
    },
  });

  return (
    <div>
      {messages.map(m => (
        <div key={m.id}>
          <strong>{m.role}:</strong>
          {m.content}
          {m.reasoning && (
            <details>
              <summary>Reasoning</summary>
              {m.reasoning}
            </details>
          )}
        </div>
      ))}
      <form onSubmit={handleSubmit}>
        <input value={input} onChange={handleInputChange} />
      </form>
    </div>
  );
}
```

---

## Model Comparison

| Model | Provider | Reasoning | Latency | Quality |
|-------|----------|-----------|---------|---------|
| Claude 3.7 Sonnet | Anthropic | Configurable thinking | Medium | Highest |
| DeepSeek-R1 | Fireworks | <think> tags | High | High |
| DeepSeek-R1 Distill | Groq | <think> tags | Low | Medium-High |

---

## Environment Variables

```bash
# .env.example
ANTHROPIC_API_KEY=sk-...
FIREWORKS_API_KEY=...
GROQ_API_KEY=...
```

---

## File Structure

```
ai-sdk-reasoning-starter/
├── app/
│   ├── api/
│   │   └── chat/
│   │       └── route.ts      # Chat API endpoint
│   ├── layout.tsx            # Root layout
│   ├── page.tsx              # Chat UI
│   └── globals.css           # Styles
├── components/
│   └── ui/                   # shadcn/ui components
├── lib/
│   └── models.ts             # Model provider configuration
├── package.json
└── tsconfig.json
```

---

## Running Locally

```bash
# Install dependencies
pnpm install

# Link with Vercel (optional)
vercel link

# Pull environment variables
vercel env pull

# Run development server
pnpm dev
```

---

## Deployment

```bash
# Deploy to Vercel
vercel deploy

# Or use the one-click deploy button
https://vercel.com/new/clone?repository-url=...
```

---

## Rust Implementation Considerations

### 1. Streaming LLM Client

For Rust, you'd need:
- Async HTTP client with SSE support (`reqwest` + SSE parser)
- Token streaming abstraction
- Model provider trait

```rust
// Hypothetical Rust API
trait LLMProvider: Send + Sync {
    async fn stream_chat(
        &self,
        messages: Vec<Message>,
        options: GenerationOptions,
    ) -> Result<impl Stream<Item = Token>>;
}

struct AnthropicProvider {
    client: Client,
    api_key: String,
}

impl LLMProvider for AnthropicProvider {
    async fn stream_chat(...) {
        // SSE streaming to Anthropic API
    }
}
```

### 2. Reasoning Extraction

```rust
// Parse <think> tags from DeepSeek
fn extract_reasoning(content: &str) -> (Option<String>, String) {
    let think_pattern = Regex::new(r"<think>(.*?)</think>(.*)").unwrap();

    if let Some(caps) = think_pattern.captures(content) {
        (Some(caps[1].to_string()), caps[2].to_string())
    } else {
        (None, content.to_string())
    }
}
```

### 3. Provider Abstraction

```rust
enum ModelProvider {
    Anthropic { model: String },
    Fireworks { account: String, model: String },
    Groq { model: String },
}

struct GenerationConfig {
    pub provider: ModelProvider,
    pub thinking_enabled: bool,
    pub thinking_budget: usize,
    pub temperature: f32,
    pub max_tokens: usize,
}
```

---

## Key Takeaways

1. **Unified Provider API** - `customProvider` abstracts multiple LLM providers
2. **Reasoning Middleware** - `extractReasoningMiddleware` handles <think> parsing
3. **Provider Options** - Anthropic-specific settings via `providerOptions`
4. **Smooth Streaming** - `smoothStream` improves UX with word-by-word output
5. **Type Safety** - TypeScript types for model IDs and configurations

---

## See Also

- [AI SDK Documentation](https://ai-sdk.dev/)
- [Anthropic Thinking](https://docs.anthropic.com/claude/docs/thinking)
- [Main Vercel Labs Exploration](./exploration.md)
