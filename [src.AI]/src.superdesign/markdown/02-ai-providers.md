# AI Providers: Vercel AI SDK

SuperDesign uses the Vercel AI SDK (`ai` package) as its abstraction layer, supporting 4 AI providers: Anthropic, OpenAI, OpenRouter, and Google.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.superdesign/superdesign/src/services/customAgentService.ts`

## Provider Abstraction

```mermaid
flowchart TD
    CONFIG["VS Code Settings
    superdesign.aiModelProvider
    superdesign.aiModel"] --> SWITCH["Provider Switch
    getModel() method"]
    SWITCH --> ANTHROPIC["createAnthropic
    @ai-sdk/anthropic
    Default: claude-3-5-sonnet-20241022"]
    SWITCH --> OPENAI["createOpenAI
    @ai-sdk/openai
    Default: gpt-4o"]
    SWITCH --> OPENROUTER["createOpenRouter
    @openrouter/ai-sdk-provider
    Default: anthropic/claude-3-7-sonnet"]
    SWITCH --> GOOGLE["createGoogle
    @ai-sdk/google"]
```

**Aha:** The provider is determined by a combination of `aiModelProvider` setting and the model name prefix. If the model contains `/`, it uses OpenRouter. If it starts with `claude-`, it uses Anthropic. Otherwise, OpenAI.

**Source:** `src/services/customAgentService.ts` lines 80-163.

## Provider Resolution Flow

When the AI provider is determined, the resolution follows a two-tier cascade:

```mermaid
flowchart TD
    ReadCfg["Read VS Code config
    aiModelProvider, aiModel"] --> HasModel{"aiModel set?"}

    HasModel -->|"Yes"| Infer{"Model name analysis"}
    HasModel -->|"No"| UseDefault["Use default for aiModelProvider"]

    Infer --> ContainsSlash{"Model contains '/'?"}
    ContainsSlash -->|"Yes"| RouteOR["OpenRouter
    e.g. google/gemini-2.5-pro"]
    ContainsSlash -->|"No"| StartsWithClaude{"Starts with 'claude-'?"}
    StartsWithClaude -->|"Yes"| RouteAnth["Anthropic
    e.g. claude-3-5-sonnet-20241022"]
    StartsWithClaude -->|"No"| RouteOpenAI["OpenAI
    e.g. gpt-4o"]

    UseDefault --> DefAnthropic["Default: claude-3-5-sonnet-20241022"]
    RouteOR --> CheckKeyOR["Validate OpenRouter API key"]
    RouteAnth --> CheckKeyAnth["Validate Anthropic API key + Helicone proxy"]
    RouteOpenAI --> CheckKeyOpenAI["Validate OpenAI API key + Helicone proxy"]
    DefAnthropic --> CheckKeyAnth

    CheckKeyOR --> CreateModel["createProvider(model)"]
    CheckKeyAnth --> CreateModel
    CheckKeyOpenAI --> CreateModel
```

**Source:** `src/services/customAgentService.ts` lines 80-163 — `getModel()` method.

## Provider Configuration

| Provider | API Key Setting | Default Model | Helicone Proxy |
|----------|----------------|---------------|----------------|
| Anthropic | `superdesign.anthropicApiKey` | `claude-3-5-sonnet-20241022` | Yes (`anthropic.helicone.ai`) |
| OpenAI | `superdesign.openaiApiKey` | `gpt-4o` | Yes (`oai.helicone.ai`) |
| OpenRouter | `superdesign.openrouterApiKey` | `anthropic/claude-3-7-sonnet-20250219` | No |
| Google | `superdesign.googleApiKey` | — | No |

**Aha:** Anthropic and OpenAI providers use Helicone as a proxy for observability and caching. The Helicone auth header is hardcoded in the SDK configuration, meaning all requests through these providers go through Helicone's proxy layer. OpenRouter connects directly without a proxy.

**Source:** `src/services/customAgentService.ts` lines 120-162.

## Provider SDK Construction Sequence

```mermaid
sequenceDiagram
    participant Query as CustomAgentService.query()
    participant GetModel as getModel()
    participant Config as vscode.workspace.getConfiguration()
    participant SDK as AI SDK Factory
    participant Model as Model Instance

    Query->>GetModel: this.getModel()
    GetModel->>Config: get('aiModelProvider')
    GetModel->>Config: get('aiModel')
    GetModel->>GetModel: Infer effectiveProvider
    alt openrouter
        GetModel->>Config: get('openrouterApiKey')
        GetModel->>SDK: createOpenRouter({apiKey})
        SDK-->>GetModel: openrouter provider
        GetModel->>Model: openrouter.chat('anthropic/claude-3-7-sonnet')
    else anthropic
        GetModel->>Config: get('anthropicApiKey')
        GetModel->>SDK: createAnthropic({apiKey, baseURL: helicone})
        SDK-->>GetModel: anthropic provider
        GetModel->>Model: anthropic('claude-3-5-sonnet-20241022')
    else openai
        GetModel->>Config: get('openaiApiKey')
        GetModel->>SDK: createOpenAI({apiKey, baseURL: helicone})
        SDK-->>GetModel: openai provider
        GetModel->>Model: openai('gpt-4o')
    end
    Model-->>Query: Model instance for streamText()
```

## Streaming

The agent uses `streamText()` from the Vercel AI SDK:

```typescript
import { streamText, CoreMessage } from 'ai';

const result = streamText({
  model: this.getModel(),
  messages: coreMessages,
  tools: this.getTools(),
  system: this.getSystemPrompt(),
});
```

This provides streaming responses with tool execution support.

## System Prompt

The system prompt varies based on the configured model:

```typescript
// src/services/customAgentService.ts
// System prompt instructs the AI to:
// - Design UI based on user prompts
// - Use available tools to create files
// - Generate clean, modern HTML/CSS
```

**Source:** `src/services/customAgentService.ts` lines 165+.

## VS Code Settings

| Setting | Type | Description |
|---------|------|-------------|
| `superdesign.aiModelProvider` | string | Provider: 'anthropic', 'openai', 'openrouter', 'google' |
| `superdesign.aiModel` | string | Specific model name (overrides default) |
| `superdesign.anthropicApiKey` | string | Anthropic API key |
| `superdesign.openaiApiKey` | string | OpenAI API key |
| `superdesign.openrouterApiKey` | string | OpenRouter API key |
| `superdesign.googleApiKey` | string | Google AI API key |

## Next

Continue to [03-tools](03-tools.md) for the 9 coding tools.
